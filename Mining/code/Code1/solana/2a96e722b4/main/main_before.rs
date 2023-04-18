fn main() -> Result<(), Box<dyn error::Error>> {
    let config = get_config();

    solana_logger::setup_with_default("solana=info");
    solana_metrics::set_panic_hook("watchtower");

    let _notify_thread = if config.notify_on_transactions {
        let rpc_client = RpcClient::new(config.json_rpc_url.clone());
        Some(std::thread::spawn(move || transaction_monitor(rpc_client)))
    } else {
        None
    };

    let rpc_client = RpcClient::new(config.json_rpc_url.clone());
    let notifier = Notifier::default();
    let mut last_transaction_count = 0;
    let mut last_recent_blockhash = Hash::default();
    let mut last_notification_msg = "".into();
    let mut last_success = Instant::now();

    loop {
        let failure = match get_cluster_info(&rpc_client) {
            Ok((transaction_count, recent_blockhash, vote_accounts)) => {
                info!("Current transaction count: {}", transaction_count);
                info!("Recent blockhash: {}", recent_blockhash);
                info!("Current validator count: {}", vote_accounts.current.len());
                info!(
                    "Delinquent validator count: {}",
                    vote_accounts.delinquent.len()
                );

                let mut failures = vec![];

                let total_current_stake = vote_accounts
                    .current
                    .iter()
                    .map(|vote_account| vote_account.activated_stake)
                    .sum();
                let total_delinquent_stake = vote_accounts
                    .delinquent
                    .iter()
                    .map(|vote_account| vote_account.activated_stake)
                    .sum();

                let total_stake = total_current_stake + total_delinquent_stake;
                let current_stake_percent = total_current_stake * 100 / total_stake;
                info!(
                    "Current stake: {}% | Total stake: {} SOL, current stake: {} SOL, delinquent: {} SOL",
                    current_stake_percent,
                    lamports_to_sol(total_stake),
                    lamports_to_sol(total_current_stake),
                    lamports_to_sol(total_delinquent_stake)
                );

                if transaction_count > last_transaction_count {
                    last_transaction_count = transaction_count;
                } else {
                    failures.push((
                        "transaction-count",
                        format!(
                            "Transaction count is not advancing: {} <= {}",
                            transaction_count, last_transaction_count
                        ),
                    ));
                }

                if recent_blockhash != last_recent_blockhash {
                    last_recent_blockhash = recent_blockhash;
                } else {
                    failures.push((
                        "recent-blockhash",
                        format!("Unable to get new blockhash: {}", recent_blockhash),
                    ));
                }

                if config.monitor_active_stake && current_stake_percent < 80 {
                    failures.push((
                        "current-stake",
                        format!("Current stake is {}%", current_stake_percent),
                    ));
                }

                if config.validator_identity_pubkeys.is_empty() {
                    if !vote_accounts.delinquent.is_empty() {
                        failures.push((
                            "delinquent",
                            format!("{} delinquent validators", vote_accounts.delinquent.len()),
                        ));
                    }
                } else {
                    let mut errors = vec![];
                    for validator_identity in config.validator_identity_pubkeys.iter() {
                        let formatted_validator_identity =
                            format_labeled_address(&validator_identity, &config.address_labels);
                        if vote_accounts
                            .delinquent
                            .iter()
                            .any(|vai| vai.node_pubkey == *validator_identity)
                        {
                            errors.push(format!("{} delinquent", formatted_validator_identity));
                        } else if !vote_accounts
                            .current
                            .iter()
                            .any(|vai| vai.node_pubkey == *validator_identity)
                        {
                            errors.push(format!("{} missing", formatted_validator_identity));
                        }

                        rpc_client
                            .get_balance(&Pubkey::from_str(&validator_identity).unwrap_or_default())
                            .map(lamports_to_sol)
                            .map(|balance| {
                                if balance < 10.0 {
                                    // At 1 SOL/day for validator voting fees, this gives over a week to
                                    // find some more SOL
                                    failures.push((
                                        "balance",
                                        format!(
                                            "{} has {} SOL",
                                            formatted_validator_identity, balance
                                        ),
                                    ));
                                }
                            })
                            .unwrap_or_else(|err| {
                                warn!(
                                    "Failed to get balance of {}: {:?}",
                                    formatted_validator_identity, err
                                );
                            });
                    }

                    if !errors.is_empty() {
                        failures.push(("delinquent", errors.join(",")));
                    }
                }

                for failure in failures.iter() {
                    error!("{} sanity failure: {}", failure.0, failure.1);
                }
                failures.into_iter().next() // Only report the first failure if any
            }
            Err(err) => Some(("rpc", err.to_string())),
        };

        datapoint_info!("watchtower-sanity", ("ok", failure.is_none(), bool));
        if let Some((failure_test_name, failure_error_message)) = &failure {
            let notification_msg = format!(
                "solana-watchtower: Error: {}: {}",
                failure_test_name, failure_error_message
            );
            if !config.no_duplicate_notifications || last_notification_msg != notification_msg {
                notifier.send(&notification_msg);
            }
            datapoint_error!(
                "watchtower-sanity-failure",
                ("test", failure_test_name, String),
                ("err", failure_error_message, String)
            );
            last_notification_msg = notification_msg;
        } else {
            if !last_notification_msg.is_empty() {
                let alarm_duration = Instant::now().duration_since(last_success);
                let alarm_duration = Duration::from_secs(alarm_duration.as_secs()); // Drop milliseconds in message

                let all_clear_msg = format!(
                    "All clear after {}",
                    humantime::format_duration(alarm_duration)
                );
                info!("{}", all_clear_msg);
                notifier.send(&format!("solana-watchtower: {}", all_clear_msg));
            }
            last_notification_msg = "".into();
            last_success = Instant::now();
        }
        sleep(config.interval);
    }
}
