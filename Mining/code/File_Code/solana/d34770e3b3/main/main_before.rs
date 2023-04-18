use clap::{crate_description, crate_name, crate_version, value_t, value_t_or_exit, App, Arg};
use log::*;
use solana_clap_utils::{
    input_parsers::{keypair_of, pubkey_of},
    input_validators::{is_amount, is_keypair, is_pubkey_or_keypair, is_url, is_valid_percentage},
};
use solana_cli_output::display::format_labeled_address;
use solana_client::{
    client_error, rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig,
    rpc_request::MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS, rpc_response::RpcVoteAccountInfo,
};
use solana_metrics::datapoint_info;
use solana_notifier::Notifier;
use solana_sdk::{
    account_utils::StateMut,
    clock::{Epoch, Slot},
    commitment_config::CommitmentConfig,
    message::Message,
    native_token::*,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use solana_stake_program::{stake_instruction, stake_state::StakeState};

use std::{
    collections::{HashMap, HashSet},
    error,
    fs::File,
    path::PathBuf,
    process,
    str::FromStr,
    thread::sleep,
    time::Duration,
};

mod validator_list;

#[derive(Debug)]
struct Config {
    json_rpc_url: String,
    cluster: String,
    source_stake_address: Pubkey,
    authorized_staker: Keypair,

    /// Only validators with an identity pubkey in this validator_list will be staked
    validator_list: HashSet<Pubkey>,

    dry_run: bool,

    /// Amount of lamports to stake any validator in the validator_list that is not delinquent
    baseline_stake_amount: u64,

    /// Amount of additional lamports to stake quality block producers in the validator_list
    bonus_stake_amount: u64,

    /// Quality validators produce a block at least this percentage of their leader slots over the
    /// previous epoch
    quality_block_producer_percentage: usize,

    /// A delinquent validator gets this number of slots of grace (from the current slot) before it
    /// will be fully destaked.  The grace period is intended to account for unexpected bugs that
    /// cause a validator to go down
    delinquent_grace_slot_distance: u64,

    /// Don't ever unstake more than this percentage of the cluster at one time
    max_poor_block_producer_percentage: usize,

    /// Vote accounts with a larger commission than this amount will not be staked.
    max_commission: u8,

    address_labels: HashMap<String, String>,
}

fn get_config() -> Config {
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(&config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster")
        )
        .arg(
            Arg::with_name("cluster")
                .long("cluster")
                .value_name("NAME")
                .possible_values(&["mainnet-beta", "testnet"])
                .takes_value(true)
                .help("Name of the cluster to operate on")
        )
        .arg(
            Arg::with_name("validator_list_file")
                .long("validator-list")
                .value_name("FILE")
                .required(true)
                .takes_value(true)
                .conflicts_with("cluster")
                .help("File containing an YAML array of validator pubkeys eligible for staking")
        )
        .arg(
            Arg::with_name("confirm")
                .long("confirm")
                .takes_value(false)
                .help("Confirm that the stake adjustments should actually be made")
        )
        .arg(
            Arg::with_name("source_stake_address")
                .index(1)
                .value_name("ADDRESS")
                .takes_value(true)
                .required(true)
                .validator(is_pubkey_or_keypair)
                .help("The source stake account for splitting individual validator stake accounts from")
        )
        .arg(
            Arg::with_name("authorized_staker")
                .index(2)
                .value_name("KEYPAIR")
                .validator(is_keypair)
                .required(true)
                .takes_value(true)
        )
        .arg(
            Arg::with_name("quality_block_producer_percentage")
                .long("quality-block-producer-percentage")
                .value_name("PERCENTAGE")
                .takes_value(true)
                .default_value("75")
                .validator(is_valid_percentage)
                .help("Quality validators produce a block in at least this percentage of their leader slots over the previous epoch")
        )
        .arg(
            Arg::with_name("max_poor_block_producer_percentage")
                .long("max-poor-block-producer-percentage")
                .value_name("PERCENTAGE")
                .takes_value(true)
                .default_value("20")
                .validator(is_valid_percentage)
                .help("Do not add or remove bonus stake from any non-delinquent validators if at least this percentage of all validators are poor block producers")
        )
        .arg(
            Arg::with_name("baseline_stake_amount")
                .long("baseline-stake-amount")
                .value_name("SOL")
                .takes_value(true)
                .default_value("5000")
                .validator(is_amount)
        )
        .arg(
            Arg::with_name("bonus_stake_amount")
                .long("bonus-stake-amount")
                .value_name("SOL")
                .takes_value(true)
                .default_value("50000")
                .validator(is_amount)
        )
        .arg(
            Arg::with_name("max_commission")
                .long("max-commission")
                .value_name("PERCENTAGE")
                .takes_value(true)
                .default_value("100")
                .validator(is_valid_percentage)
                .help("Vote accounts with a larger commission than this amount will not be staked")
        )
        .get_matches();

    let config = if let Some(config_file) = matches.value_of("config_file") {
        solana_cli_config::Config::load(config_file).unwrap_or_default()
    } else {
        solana_cli_config::Config::default()
    };

    let source_stake_address = pubkey_of(&matches, "source_stake_address").unwrap();
    let authorized_staker = keypair_of(&matches, "authorized_staker").unwrap();
    let dry_run = !matches.is_present("confirm");
    let cluster = value_t!(matches, "cluster", String).unwrap_or_else(|_| "unknown".into());
    let quality_block_producer_percentage =
        value_t_or_exit!(matches, "quality_block_producer_percentage", usize);
    let max_commission = value_t_or_exit!(matches, "max_commission", u8);
    let max_poor_block_producer_percentage =
        value_t_or_exit!(matches, "max_poor_block_producer_percentage", usize);
    let baseline_stake_amount =
        sol_to_lamports(value_t_or_exit!(matches, "baseline_stake_amount", f64));
    let bonus_stake_amount = sol_to_lamports(value_t_or_exit!(matches, "bonus_stake_amount", f64));

    let (json_rpc_url, validator_list) = match cluster.as_str() {
        "mainnet-beta" => (
            value_t!(matches, "json_rpc_url", String)
                .unwrap_or_else(|_| "http://api.mainnet-beta.solana.com".into()),
            validator_list::mainnet_beta_validators(),
        ),
        "testnet" => (
            value_t!(matches, "json_rpc_url", String)
                .unwrap_or_else(|_| "http://testnet.solana.com".into()),
            validator_list::testnet_validators(),
        ),
        "unknown" => {
            let validator_list_file =
                File::open(value_t_or_exit!(matches, "validator_list_file", PathBuf))
                    .unwrap_or_else(|err| {
                        error!("Unable to open validator_list: {}", err);
                        process::exit(1);
                    });

            let validator_list = serde_yaml::from_reader::<_, Vec<String>>(validator_list_file)
                .unwrap_or_else(|err| {
                    error!("Unable to read validator_list: {}", err);
                    process::exit(1);
                })
                .into_iter()
                .map(|p| {
                    Pubkey::from_str(&p).unwrap_or_else(|err| {
                        error!("Invalid validator_list pubkey '{}': {}", p, err);
                        process::exit(1);
                    })
                })
                .collect();
            (
                value_t!(matches, "json_rpc_url", String)
                    .unwrap_or_else(|_| config.json_rpc_url.clone()),
                validator_list,
            )
        }
        _ => unreachable!(),
    };
    let validator_list = validator_list.into_iter().collect::<HashSet<_>>();

    let config = Config {
        json_rpc_url,
        cluster,
        source_stake_address,
        authorized_staker,
        validator_list,
        dry_run,
        baseline_stake_amount,
        bonus_stake_amount,
        delinquent_grace_slot_distance: 21600, // ~24 hours worth of slots at 2.5 slots per second
        quality_block_producer_percentage,
        max_commission,
        max_poor_block_producer_percentage,
        address_labels: config.address_labels,
    };

    info!("RPC URL: {}", config.json_rpc_url);
    config
}

fn get_stake_account(
    rpc_client: &RpcClient,
    address: &Pubkey,
) -> Result<(u64, StakeState), String> {
    let account = rpc_client.get_account(address).map_err(|e| {
        format!(
            "Failed to fetch stake account {}: {}",
            address,
            e.to_string()
        )
    })?;

    if account.owner != solana_stake_program::id() {
        return Err(format!(
            "not a stake account (owned by {}): {}",
            account.owner, address
        ));
    }

    account
        .state()
        .map_err(|e| {
            format!(
                "Failed to decode stake account at {}: {}",
                address,
                e.to_string()
            )
        })
        .map(|stake_state| (account.lamports, stake_state))
}

fn retry_rpc_operation<T, F>(mut retries: usize, op: F) -> client_error::Result<T>
where
    F: Fn() -> client_error::Result<T>,
{
    loop {
        let result = op();

        if let Err(client_error::ClientError {
            kind: client_error::ClientErrorKind::Reqwest(ref reqwest_error),
            ..
        }) = result
        {
            if reqwest_error.is_timeout() && retries > 0 {
                info!("RPC request timeout, {} retries remaining", retries);
                retries -= 1;
                continue;
            }
        }
        return result;
    }
}

/// Split validators into quality/poor lists based on their block production over the given `epoch`
fn classify_block_producers(
    rpc_client: &RpcClient,
    config: &Config,
    epoch: Epoch,
) -> Result<(HashSet<Pubkey>, HashSet<Pubkey>), Box<dyn error::Error>> {
    let epoch_schedule = rpc_client.get_epoch_schedule()?;
    let first_slot_in_epoch = epoch_schedule.get_first_slot_in_epoch(epoch);
    let last_slot_in_epoch = epoch_schedule.get_last_slot_in_epoch(epoch);

    let first_available_block = rpc_client.get_first_available_block()?;
    let minimum_ledger_slot = rpc_client.minimum_ledger_slot()?;
    debug!(
        "first_available_block: {}, minimum_ledger_slot: {}",
        first_available_block, minimum_ledger_slot
    );

    if first_available_block >= last_slot_in_epoch {
        return Err(format!(
            "First available block is newer than the last epoch: {} > {}",
            first_available_block, last_slot_in_epoch
        )
        .into());
    }

    let first_slot = if first_available_block > first_slot_in_epoch {
        first_available_block
    } else {
        first_slot_in_epoch
    };

    let leader_schedule = rpc_client.get_leader_schedule(Some(first_slot))?.unwrap();

    let mut confirmed_blocks = vec![];
    // Fetching a large number of blocks from BigTable can cause timeouts, break up the requests
    const LONGTERM_STORAGE_STEP: u64 = 5_000;
    let mut next_slot = first_slot;
    while next_slot < last_slot_in_epoch {
        let last_slot = if next_slot >= minimum_ledger_slot {
            last_slot_in_epoch
        } else {
            last_slot_in_epoch.min(next_slot + LONGTERM_STORAGE_STEP)
        };
        let slots_remaining = last_slot_in_epoch - last_slot;
        info!(
            "Fetching confirmed blocks between {} - {}{}",
            next_slot,
            last_slot,
            if slots_remaining > 0 {
                format!(" ({} remaining)", slots_remaining)
            } else {
                "".to_string()
            }
        );

        confirmed_blocks.push(retry_rpc_operation(42, || {
            rpc_client.get_confirmed_blocks(next_slot, Some(last_slot))
        })?);
        next_slot = last_slot + 1;
    }
    let confirmed_blocks: HashSet<Slot> = confirmed_blocks.into_iter().flatten().collect();

    let mut poor_block_producers = HashSet::new();
    let mut quality_block_producers = HashSet::new();

    for (validator_identity, relative_slots) in leader_schedule {
        let mut validator_blocks = 0;
        let mut validator_slots = 0;
        for relative_slot in relative_slots {
            let slot = first_slot_in_epoch + relative_slot as Slot;
            if slot >= first_slot {
                validator_slots += 1;
                if confirmed_blocks.contains(&slot) {
                    validator_blocks += 1;
                }
            }
        }
        trace!(
            "Validator {} produced {} blocks in {} slots",
            validator_identity,
            validator_blocks,
            validator_slots
        );
        if validator_slots > 0 {
            let validator_identity = Pubkey::from_str(&validator_identity)?;
            if validator_blocks * 100 / validator_slots >= config.quality_block_producer_percentage
            {
                quality_block_producers.insert(validator_identity);
            } else {
                poor_block_producers.insert(validator_identity);
            }
        }
    }

    info!("quality_block_producers: {}", quality_block_producers.len());
    trace!("quality_block_producers: {:?}", quality_block_producers);
    info!("poor_block_producers: {}", poor_block_producers.len());
    trace!("poor_block_producers: {:?}", poor_block_producers);
    Ok((quality_block_producers, poor_block_producers))
}

fn validate_source_stake_account(
    rpc_client: &RpcClient,
    config: &Config,
) -> Result<u64, Box<dyn error::Error>> {
    // check source stake account
    let (source_stake_balance, source_stake_state) =
        get_stake_account(&rpc_client, &config.source_stake_address)?;

    info!(
        "stake account balance: {} SOL",
        lamports_to_sol(source_stake_balance)
    );
    match &source_stake_state {
        StakeState::Initialized(_) | StakeState::Stake(_, _) => source_stake_state
            .authorized()
            .map_or(Ok(source_stake_balance), |authorized| {
                if authorized.staker != config.authorized_staker.pubkey() {
                    Err(format!(
                        "The authorized staker for the source stake account is not {}",
                        config.authorized_staker.pubkey()
                    )
                    .into())
                } else {
                    Ok(source_stake_balance)
                }
            }),
        _ => Err(format!(
            "Source stake account is not in the initialized state: {:?}",
            source_stake_state
        )
        .into()),
    }
}

struct ConfirmedTransaction {
    success: bool,
    signature: Signature,
    memo: String,
}

/// Simulate a list of transactions and filter out the ones that will fail
fn simulate_transactions(
    rpc_client: &RpcClient,
    candidate_transactions: Vec<(Transaction, String)>,
) -> client_error::Result<Vec<(Transaction, String)>> {
    info!("Simulating {} transactions", candidate_transactions.len(),);
    let mut simulated_transactions = vec![];
    for (mut transaction, memo) in candidate_transactions {
        transaction.message.recent_blockhash =
            retry_rpc_operation(10, || rpc_client.get_recent_blockhash())?.0;

        let sim_result = rpc_client.simulate_transaction_with_config(
            &transaction,
            RpcSimulateTransactionConfig {
                sig_verify: false,
                ..RpcSimulateTransactionConfig::default()
            },
        )?;
        if sim_result.value.err.is_some() {
            trace!(
                "filtering out transaction due to simulation failure: {:?}: {}",
                sim_result,
                memo
            );
        } else {
            simulated_transactions.push((transaction, memo))
        }
    }
    info!(
        "Successfully simulating {} transactions",
        simulated_transactions.len()
    );
    Ok(simulated_transactions)
}

fn transact(
    rpc_client: &RpcClient,
    dry_run: bool,
    transactions: Vec<(Transaction, String)>,
    authorized_staker: &Keypair,
) -> Result<Vec<ConfirmedTransaction>, Box<dyn error::Error>> {
    let authorized_staker_balance = rpc_client.get_balance(&authorized_staker.pubkey())?;
    info!(
        "Authorized staker balance: {} SOL",
        lamports_to_sol(authorized_staker_balance)
    );

    let (blockhash, fee_calculator, last_valid_slot) = rpc_client
        .get_recent_blockhash_with_commitment(rpc_client.commitment())?
        .value;
    info!("{} transactions to send", transactions.len());

    let required_fee = transactions.iter().fold(0, |fee, (transaction, _)| {
        fee + fee_calculator.calculate_fee(&transaction.message)
    });
    info!("Required fee: {} SOL", lamports_to_sol(required_fee));
    if required_fee > authorized_staker_balance {
        return Err("Authorized staker has insufficient funds".into());
    }

    let mut pending_transactions = HashMap::new();
    for (mut transaction, memo) in transactions.into_iter() {
        transaction.sign(&[authorized_staker], blockhash);

        pending_transactions.insert(transaction.signatures[0], memo);
        if !dry_run {
            rpc_client.send_transaction(&transaction)?;
        }
    }

    let mut finalized_transactions = vec![];
    loop {
        if pending_transactions.is_empty() {
            break;
        }

        let slot = rpc_client.get_slot_with_commitment(CommitmentConfig::finalized())?;
        info!(
            "Current slot={}, last_valid_slot={} (slots remaining: {}) ",
            slot,
            last_valid_slot,
            last_valid_slot.saturating_sub(slot)
        );

        if slot > last_valid_slot {
            error!(
                "Blockhash {} expired with {} pending transactions",
                blockhash,
                pending_transactions.len()
            );

            for (signature, memo) in pending_transactions.into_iter() {
                finalized_transactions.push(ConfirmedTransaction {
                    success: false,
                    signature,
                    memo,
                });
            }
            break;
        }

        let pending_signatures = pending_transactions.keys().cloned().collect::<Vec<_>>();
        let mut statuses = vec![];
        for pending_signatures_chunk in
            pending_signatures.chunks(MAX_GET_SIGNATURE_STATUSES_QUERY_ITEMS - 1)
        {
            trace!(
                "checking {} pending_signatures",
                pending_signatures_chunk.len()
            );
            statuses.extend(
                rpc_client
                    .get_signature_statuses(&pending_signatures_chunk)?
                    .value
                    .into_iter(),
            )
        }
        assert_eq!(statuses.len(), pending_signatures.len());

        for (signature, status) in pending_signatures.into_iter().zip(statuses.into_iter()) {
            info!("{}: status={:?}", signature, status);
            let completed = if dry_run {
                Some(true)
            } else if let Some(status) = &status {
                if status.confirmations.is_none() || status.err.is_some() {
                    Some(status.err.is_none())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(success) = completed {
                warn!("{}: completed.  success={}", signature, success);
                let memo = pending_transactions.remove(&signature).unwrap();
                finalized_transactions.push(ConfirmedTransaction {
                    success,
                    signature,
                    memo,
                });
            }
        }
        sleep(Duration::from_secs(5));
    }

    Ok(finalized_transactions)
}

fn process_confirmations(
    mut confirmations: Vec<ConfirmedTransaction>,
    notifier: Option<&Notifier>,
) -> bool {
    let mut ok = true;

    confirmations.sort_by(|a, b| a.memo.cmp(&b.memo));
    for ConfirmedTransaction {
        success,
        signature,
        memo,
    } in confirmations
    {
        if success {
            info!("OK:   {}: {}", signature, memo);
            if let Some(notifier) = notifier {
                notifier.send(&memo)
            }
        } else {
            error!("FAIL: {}: {}", signature, memo);
            ok = false
        }
    }
    ok
}

#[allow(clippy::cognitive_complexity)] // Yeah I know...
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
