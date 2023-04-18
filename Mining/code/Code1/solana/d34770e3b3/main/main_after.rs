fn main() -> Result<(), Box<dyn error::Error>> {
    solana_logger::setup_with_default("solana=info");
    let config = get_config();

    let notifier = Notifier::default();
    let rpc_client = RpcClient::new(config.json_rpc_url.clone());

    if !config.dry_run && notifier.is_empty() {
        error!("A notifier must be active with --confirm");
        process::exit(1);
    }

    let source_stake_balance = validate_source_stake_account(&rpc_client, &config)?;

    let epoch_info = rpc_client.get_epoch_info()?;
    let last_epoch = epoch_info.epoch - 1;

    info!("Epoch info: {:?}", epoch_info);

    let (quality_block_producers, poor_block_producers) =
        classify_block_producers(&rpc_client, &config, last_epoch)?;

    let too_many_poor_block_producers = poor_block_producers.len()
        > quality_block_producers.len() * config.max_poor_block_producer_percentage / 100;

    // Fetch vote account status for all the validator_listed validators
    let vote_account_status = rpc_client.get_vote_accounts()?;
    let vote_account_info = vote_account_status
        .current
        .into_iter()
        .chain(vote_account_status.delinquent.into_iter())
        .filter_map(|vai| {
            let node_pubkey = Pubkey::from_str(&vai.node_pubkey).ok()?;
            if config.validator_list.contains(&node_pubkey) {
                Some(vai)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut source_stake_lamports_required = 0;
    let mut create_stake_transactions = vec![];
    let mut delegate_stake_transactions = vec![];
    let mut stake_activated_in_current_epoch: HashSet<Pubkey> = HashSet::new();

    for RpcVoteAccountInfo {
        commission,
        node_pubkey,
        root_slot,
        vote_pubkey,
        ..
    } in &vote_account_info
    {
        let formatted_node_pubkey = format_labeled_address(&node_pubkey, &config.address_labels);
        let node_pubkey = Pubkey::from_str(&node_pubkey).unwrap();
        let baseline_seed = &vote_pubkey.to_string()[..32];
        let bonus_seed = &format!("A{{{}", vote_pubkey)[..32];
        let vote_pubkey = Pubkey::from_str(&vote_pubkey).unwrap();

        let baseline_stake_address = Pubkey::create_with_seed(
            &config.authorized_staker.pubkey(),
            baseline_seed,
            &solana_stake_program::id(),
        )
        .unwrap();
        let bonus_stake_address = Pubkey::create_with_seed(
            &config.authorized_staker.pubkey(),
            bonus_seed,
            &solana_stake_program::id(),
        )
        .unwrap();

        debug!(
            "\nidentity: {}\n - vote address: {}\n - baseline stake: {}\n - bonus stake: {}",
            node_pubkey, vote_pubkey, baseline_stake_address, bonus_stake_address
        );

        // Transactions to create the baseline and bonus stake accounts
        if let Ok((balance, stake_state)) = get_stake_account(&rpc_client, &baseline_stake_address)
        {
            if balance <= config.baseline_stake_amount {
                info!(
                    "Unexpected balance in stake account {}: {}, expected {}",
                    baseline_stake_address, balance, config.baseline_stake_amount
                );
            }
            if let Some(delegation) = stake_state.delegation() {
                if epoch_info.epoch == delegation.activation_epoch {
                    stake_activated_in_current_epoch.insert(baseline_stake_address);
                }
            }
        } else {
            info!(
                "Need to create baseline stake account for validator {}",
                formatted_node_pubkey
            );
            source_stake_lamports_required += config.baseline_stake_amount;
            create_stake_transactions.push((
                Transaction::new_unsigned(Message::new(
                    &stake_instruction::split_with_seed(
                        &config.source_stake_address,
                        &config.authorized_staker.pubkey(),
                        config.baseline_stake_amount,
                        &baseline_stake_address,
                        &config.authorized_staker.pubkey(),
                        baseline_seed,
                    ),
                    Some(&config.authorized_staker.pubkey()),
                )),
                format!(
                    "Creating baseline stake account for validator {} ({})",
                    formatted_node_pubkey, baseline_stake_address
                ),
            ));
        }

        if let Ok((balance, stake_state)) = get_stake_account(&rpc_client, &bonus_stake_address) {
            if balance <= config.bonus_stake_amount {
                info!(
                    "Unexpected balance in stake account {}: {}, expected {}",
                    bonus_stake_address, balance, config.bonus_stake_amount
                );
            }
            if let Some(delegation) = stake_state.delegation() {
                if epoch_info.epoch == delegation.activation_epoch {
                    stake_activated_in_current_epoch.insert(bonus_stake_address);
                }
            }
        } else {
            info!(
                "Need to create bonus stake account for validator {}",
                formatted_node_pubkey
            );
            source_stake_lamports_required += config.bonus_stake_amount;
            create_stake_transactions.push((
                Transaction::new_unsigned(Message::new(
                    &stake_instruction::split_with_seed(
                        &config.source_stake_address,
                        &config.authorized_staker.pubkey(),
                        config.bonus_stake_amount,
                        &bonus_stake_address,
                        &config.authorized_staker.pubkey(),
                        bonus_seed,
                    ),
                    Some(&config.authorized_staker.pubkey()),
                )),
                format!(
                    "Creating bonus stake account for validator {} ({})",
                    formatted_node_pubkey, bonus_stake_address
                ),
            ));
        }

        if *commission > config.max_commission {
            // Deactivate baseline stake
            delegate_stake_transactions.push((
                Transaction::new_unsigned(Message::new(
                    &[stake_instruction::deactivate_stake(
                        &baseline_stake_address,
                        &config.authorized_staker.pubkey(),
                    )],
                    Some(&config.authorized_staker.pubkey()),
                )),
                format!(
                    "⛔ `{}` commission of {}% is too high. Max commission is {}%. Removed ◎{} baseline stake",
                    formatted_node_pubkey,
                    commission,
                    config.max_commission,
                    lamports_to_sol(config.baseline_stake_amount),
                ),
            ));

            // Deactivate bonus stake
            delegate_stake_transactions.push((
                Transaction::new_unsigned(Message::new(
                    &[stake_instruction::deactivate_stake(
                        &bonus_stake_address,
                        &config.authorized_staker.pubkey(),
                    )],
                    Some(&config.authorized_staker.pubkey()),
                )),
                format!(
                    "⛔ `{}` commission of {}% is too high. Max commission is {}%. Removed ◎{} bonus stake",
                    formatted_node_pubkey,
                    commission,
                    config.max_commission,
                    lamports_to_sol(config.bonus_stake_amount),
                ),
            ));

        // Validator is not considered delinquent if its root slot is less than 256 slots behind the current
        // slot.  This is very generous.
        } else if *root_slot > epoch_info.absolute_slot - 256 {
            datapoint_info!(
                "validator-status",
                ("cluster", config.cluster, String),
                ("id", node_pubkey.to_string(), String),
                ("slot", epoch_info.absolute_slot, i64),
                ("ok", true, bool)
            );

            // Delegate baseline stake
            if !stake_activated_in_current_epoch.contains(&baseline_stake_address) {
                delegate_stake_transactions.push((
                    Transaction::new_unsigned(Message::new(
                        &[stake_instruction::delegate_stake(
                            &baseline_stake_address,
                            &config.authorized_staker.pubkey(),
                            &vote_pubkey,
                        )],
                        Some(&config.authorized_staker.pubkey()),
                    )),
                    format!(
                        "🥩 `{}` is current. Added ◎{} baseline stake",
                        formatted_node_pubkey,
                        lamports_to_sol(config.baseline_stake_amount),
                    ),
                ));
            }

            if !too_many_poor_block_producers {
                if quality_block_producers.contains(&node_pubkey) {
                    // Delegate bonus stake
                    if !stake_activated_in_current_epoch.contains(&bonus_stake_address) {
                        delegate_stake_transactions.push((
                        Transaction::new_unsigned(
                        Message::new(
                            &[stake_instruction::delegate_stake(
                                &bonus_stake_address,
                                &config.authorized_staker.pubkey(),
                                &vote_pubkey,
                            )],
                            Some(&config.authorized_staker.pubkey()),
                        )),
                        format!(
                            "🏅 `{}` was a quality block producer during epoch {}. Added ◎{} bonus stake",
                            formatted_node_pubkey,
                            last_epoch,
                            lamports_to_sol(config.bonus_stake_amount),
                        ),
                    ));
                    }
                } else {
                    // Deactivate bonus stake
                    delegate_stake_transactions.push((
                    Transaction::new_unsigned(
                    Message::new(
                        &[stake_instruction::deactivate_stake(
                            &bonus_stake_address,
                            &config.authorized_staker.pubkey(),
                        )],
                        Some(&config.authorized_staker.pubkey()),
                    )),
                    format!(
                        "💔 `{}` was a poor block producer during epoch {}. Removed ◎{} bonus stake",
                        formatted_node_pubkey,
                        last_epoch,
                        lamports_to_sol(config.bonus_stake_amount),
                    ),
                ));
                }
            }
        } else {
            // Destake the validator if it has been delinquent for longer than the grace period
            if *root_slot
                < epoch_info
                    .absolute_slot
                    .saturating_sub(config.delinquent_grace_slot_distance)
            {
                // Deactivate baseline stake
                delegate_stake_transactions.push((
                    Transaction::new_unsigned(Message::new(
                        &[stake_instruction::deactivate_stake(
                            &baseline_stake_address,
                            &config.authorized_staker.pubkey(),
                        )],
                        Some(&config.authorized_staker.pubkey()),
                    )),
                    format!(
                        "🏖️ `{}` is delinquent. Removed ◎{} baseline stake",
                        formatted_node_pubkey,
                        lamports_to_sol(config.baseline_stake_amount),
                    ),
                ));

                // Deactivate bonus stake
                delegate_stake_transactions.push((
                    Transaction::new_unsigned(Message::new(
                        &[stake_instruction::deactivate_stake(
                            &bonus_stake_address,
                            &config.authorized_staker.pubkey(),
                        )],
                        Some(&config.authorized_staker.pubkey()),
                    )),
                    format!(
                        "🏖️ `{}` is delinquent. Removed ◎{} bonus stake",
                        formatted_node_pubkey,
                        lamports_to_sol(config.bonus_stake_amount),
                    ),
                ));

                datapoint_info!(
                    "validator-status",
                    ("cluster", config.cluster, String),
                    ("id", node_pubkey.to_string(), String),
                    ("slot", epoch_info.absolute_slot, i64),
                    ("ok", false, bool)
                );
            } else {
                // The validator is still considered current for the purposes of metrics reporting,
                datapoint_info!(
                    "validator-status",
                    ("cluster", config.cluster, String),
                    ("id", node_pubkey.to_string(), String),
                    ("slot", epoch_info.absolute_slot, i64),
                    ("ok", true, bool)
                );
            }
        }
    }

    if create_stake_transactions.is_empty() {
        info!("All stake accounts exist");
    } else {
        info!(
            "{} SOL is required to create {} stake accounts",
            lamports_to_sol(source_stake_lamports_required),
            create_stake_transactions.len()
        );
        if source_stake_balance < source_stake_lamports_required {
            error!(
                "Source stake account has insufficient balance: {} SOL, but {} SOL is required",
                lamports_to_sol(source_stake_balance),
                lamports_to_sol(source_stake_lamports_required)
            );
            process::exit(1);
        }

        let create_stake_transactions =
            simulate_transactions(&rpc_client, create_stake_transactions)?;
        let confirmations = transact(
            &rpc_client,
            config.dry_run,
            create_stake_transactions,
            &config.authorized_staker,
        )?;

        if !process_confirmations(confirmations, None) {
            error!("Failed to create one or more stake accounts.  Unable to continue");
            process::exit(1);
        }
    }

    let delegate_stake_transactions =
        simulate_transactions(&rpc_client, delegate_stake_transactions)?;
    let confirmations = transact(
        &rpc_client,
        config.dry_run,
        delegate_stake_transactions,
        &config.authorized_staker,
    )?;

    if too_many_poor_block_producers {
        let message = format!(
            "Note: Something is wrong, more than {}% of validators classified \
                       as poor block producers in epoch {}.  Bonus stake frozen",
            config.max_poor_block_producer_percentage, last_epoch,
        );
        warn!("{}", message);
        if !config.dry_run {
            notifier.send(&message);
        }
    }

    if !process_confirmations(
        confirmations,
        if config.dry_run {
            None
        } else {
            Some(&notifier)
        },
    ) {
        process::exit(1);
    }

    Ok(())
}
