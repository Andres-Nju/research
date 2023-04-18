use clap::{
    crate_description, crate_name, value_t, value_t_or_exit, values_t_or_exit, App, Arg,
    ArgMatches, SubCommand,
};
use regex::Regex;
use serde_json::json;
use solana_clap_utils::input_validators::{is_parsable, is_slot};
use solana_ledger::entry::Entry;
use solana_ledger::{
    ancestor_iterator::AncestorIterator,
    bank_forks_utils,
    blockstore::Blockstore,
    blockstore_db::{self, AccessType, Column, Database},
    blockstore_processor::ProcessOptions,
    rooted_slot_iterator::RootedSlotIterator,
};
use solana_runtime::{
    bank::Bank,
    bank_forks::{BankForks, CompressionType, SnapshotConfig},
    hardened_unpack::{open_genesis_config, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE},
    snapshot_utils,
    snapshot_utils::SnapshotVersion,
};
use solana_sdk::{
    clock::Slot, genesis_config::GenesisConfig, hash::Hash, native_token::lamports_to_sol,
    pubkey::Pubkey, shred_version::compute_shred_version,
};
use solana_vote_program::vote_state::VoteState;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    convert::TryInto,
    ffi::OsStr,
    fs::{self, File},
    io::{self, stdout, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{exit, Command, Stdio},
    str::FromStr,
    sync::Arc,
};

use log::*;

#[derive(PartialEq)]
enum LedgerOutputMethod {
    Print,
    Json,
}

fn output_slot_rewards(
    blockstore: &Blockstore,
    slot: Slot,
    method: &LedgerOutputMethod,
) -> Result<(), String> {
    // Note: rewards are not output in JSON yet
    if *method == LedgerOutputMethod::Print {
        if let Ok(rewards) = blockstore.read_rewards(slot) {
            if let Some(rewards) = rewards {
                if !rewards.is_empty() {
                    println!("  Rewards:");
                    for reward in rewards {
                        println!(
                            "    Account {}: {}{} SOL",
                            reward.pubkey,
                            if reward.lamports < 0 { '-' } else { ' ' },
                            lamports_to_sol(reward.lamports.abs().try_into().unwrap())
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn output_entry(
    blockstore: &Blockstore,
    method: &LedgerOutputMethod,
    slot: Slot,
    entry_index: usize,
    entry: &Entry,
) {
    match method {
        LedgerOutputMethod::Print => {
            println!(
                "  Entry {} - num_hashes: {}, hashes: {}, transactions: {}",
                entry_index,
                entry.num_hashes,
                entry.hash,
                entry.transactions.len()
            );
            for (transactions_index, transaction) in entry.transactions.iter().enumerate() {
                println!("    Transaction {}", transactions_index);
                let transaction_status = blockstore
                    .read_transaction_status((transaction.signatures[0], slot))
                    .unwrap_or_else(|err| {
                        eprintln!(
                            "Failed to read transaction status for {} at slot {}: {}",
                            transaction.signatures[0], slot, err
                        );
                        None
                    })
                    .map(|transaction_status| transaction_status.into());

                solana_cli::display::println_transaction(
                    &transaction,
                    &transaction_status,
                    "      ",
                );
            }
        }
        LedgerOutputMethod::Json => {
            // Note: transaction status is not output in JSON yet
            serde_json::to_writer(stdout(), &entry).expect("serialize entry");
            stdout().write_all(b",\n").expect("newline");
        }
    }
}

fn output_slot(
    blockstore: &Blockstore,
    slot: Slot,
    allow_dead_slots: bool,
    method: &LedgerOutputMethod,
    verbose_level: u64,
) -> Result<(), String> {
    if blockstore.is_dead(slot) {
        if allow_dead_slots {
            if *method == LedgerOutputMethod::Print {
                println!(" Slot is dead");
            }
        } else {
            return Err("Dead slot".to_string());
        }
    }

    let (entries, num_shreds, _is_full) = blockstore
        .get_slot_entries_with_shred_info(slot, 0, allow_dead_slots)
        .map_err(|err| format!("Failed to load entries for slot {}: {:?}", slot, err))?;

    if *method == LedgerOutputMethod::Print {
        if let Ok(Some(meta)) = blockstore.meta(slot) {
            if verbose_level >= 2 {
                println!(" Slot Meta {:?}", meta);
            } else {
                println!(
                    " num_shreds: {} parent_slot: {} num_entries: {}",
                    num_shreds,
                    meta.parent_slot,
                    entries.len()
                );
            }
        }
    }

    if verbose_level >= 2 {
        for (entry_index, entry) in entries.iter().enumerate() {
            output_entry(blockstore, method, slot, entry_index, entry);
        }

        output_slot_rewards(blockstore, slot, method)?;
    } else if verbose_level >= 1 {
        let mut transactions = 0;
        let mut hashes = 0;
        let mut program_ids = HashMap::new();
        for entry in &entries {
            transactions += entry.transactions.len();
            hashes += entry.num_hashes;
            for transaction in &entry.transactions {
                for instruction in &transaction.message().instructions {
                    let program_id =
                        transaction.message().account_keys[instruction.program_id_index as usize];
                    *program_ids.entry(program_id).or_insert(0) += 1;
                }
            }
        }

        let hash = if let Some(entry) = entries.last() {
            entry.hash
        } else {
            Hash::default()
        };
        println!(
            "  Transactions: {} hashes: {} block_hash: {}",
            transactions, hashes, hash,
        );
        println!("  Programs: {:?}", program_ids);
    }
    Ok(())
}

fn output_ledger(
    blockstore: Blockstore,
    starting_slot: Slot,
    allow_dead_slots: bool,
    method: LedgerOutputMethod,
    num_slots: Option<Slot>,
    verbose_level: u64,
    only_rooted: bool,
) {
    let slot_iterator = blockstore
        .slot_meta_iterator(starting_slot)
        .unwrap_or_else(|err| {
            eprintln!(
                "Failed to load entries starting from slot {}: {:?}",
                starting_slot, err
            );
            exit(1);
        });

    if method == LedgerOutputMethod::Json {
        stdout().write_all(b"{\"ledger\":[\n").expect("open array");
    }

    let num_slots = num_slots.unwrap_or(std::u64::MAX);
    let mut num_printed = 0;
    for (slot, slot_meta) in slot_iterator {
        if only_rooted && !blockstore.is_root(slot) {
            continue;
        }

        match method {
            LedgerOutputMethod::Print => {
                println!("Slot {} root?: {}", slot, blockstore.is_root(slot))
            }
            LedgerOutputMethod::Json => {
                serde_json::to_writer(stdout(), &slot_meta).expect("serialize slot_meta");
                stdout().write_all(b",\n").expect("newline");
            }
        }

        if let Err(err) = output_slot(&blockstore, slot, allow_dead_slots, &method, verbose_level) {
            eprintln!("{}", err);
        }
        num_printed += 1;
        if num_printed >= num_slots as usize {
            break;
        }
    }

    if method == LedgerOutputMethod::Json {
        stdout().write_all(b"\n]}\n").expect("close array");
    }
}

fn render_dot(dot: String, output_file: &str, output_format: &str) -> io::Result<()> {
    let mut child = Command::new("dot")
        .arg(format!("-T{}", output_format))
        .arg(format!("-o{}", output_file))
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|err| {
            eprintln!("Failed to spawn dot: {:?}", err);
            err
        })?;

    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(&dot.into_bytes())?;

    let status = child.wait_with_output()?.status;
    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("dot failed with error {}", status.code().unwrap_or(-1)),
        ));
    }
    Ok(())
}

#[allow(clippy::cognitive_complexity)]
fn graph_forks(bank_forks: &BankForks, include_all_votes: bool) -> String {
    let frozen_banks = bank_forks.frozen_banks();
    let mut fork_slots: HashSet<_> = frozen_banks.keys().cloned().collect();
    for (_, bank) in frozen_banks {
        for parent in bank.parents() {
            fork_slots.remove(&parent.slot());
        }
    }

    // Search all forks and collect the last vote made by each validator
    let mut last_votes = HashMap::new();
    for fork_slot in &fork_slots {
        let bank = &bank_forks[*fork_slot];

        let total_stake = bank
            .vote_accounts()
            .iter()
            .map(|(_, (stake, _))| stake)
            .sum();
        for (_, (stake, vote_account)) in bank.vote_accounts() {
            let vote_state = VoteState::from(&vote_account).unwrap_or_default();
            if let Some(last_vote) = vote_state.votes.iter().last() {
                let entry = last_votes.entry(vote_state.node_pubkey).or_insert((
                    last_vote.slot,
                    vote_state.clone(),
                    stake,
                    total_stake,
                ));
                if entry.0 < last_vote.slot {
                    *entry = (last_vote.slot, vote_state, stake, total_stake);
                }
            }
        }
    }

    // Figure the stake distribution at all the nodes containing the last vote from each
    // validator
    let mut slot_stake_and_vote_count = HashMap::new();
    for (last_vote_slot, _, stake, total_stake) in last_votes.values() {
        let entry = slot_stake_and_vote_count
            .entry(last_vote_slot)
            .or_insert((0, 0, *total_stake));
        entry.0 += 1;
        entry.1 += stake;
        assert_eq!(entry.2, *total_stake)
    }

    let mut dot = vec!["digraph {".to_string()];

    // Build a subgraph consisting of all banks and links to their parent banks
    dot.push("  subgraph cluster_banks {".to_string());
    dot.push("    style=invis".to_string());
    let mut styled_slots = HashSet::new();
    let mut all_votes: HashMap<Pubkey, HashMap<Slot, VoteState>> = HashMap::new();
    for fork_slot in &fork_slots {
        let mut bank = bank_forks[*fork_slot].clone();

        let mut first = true;
        loop {
            for (_, (_, vote_account)) in bank.vote_accounts() {
                let vote_state = VoteState::from(&vote_account).unwrap_or_default();
                if let Some(last_vote) = vote_state.votes.iter().last() {
                    let validator_votes = all_votes.entry(vote_state.node_pubkey).or_default();
                    validator_votes
                        .entry(last_vote.slot)
                        .or_insert_with(|| vote_state.clone());
                }
            }

            if !styled_slots.contains(&bank.slot()) {
                dot.push(format!(
                    r#"    "{}"[label="{} (epoch {})\nleader: {}{}{}",style="{}{}"];"#,
                    bank.slot(),
                    bank.slot(),
                    bank.epoch(),
                    bank.collector_id(),
                    if let Some(parent) = bank.parent() {
                        format!(
                            "\ntransactions: {}",
                            bank.transaction_count() - parent.transaction_count(),
                        )
                    } else {
                        "".to_string()
                    },
                    if let Some((votes, stake, total_stake)) =
                        slot_stake_and_vote_count.get(&bank.slot())
                    {
                        format!(
                            "\nvotes: {}, stake: {:.1} SOL ({:.1}%)",
                            votes,
                            lamports_to_sol(*stake),
                            *stake as f64 / *total_stake as f64 * 100.,
                        )
                    } else {
                        "".to_string()
                    },
                    if first { "filled," } else { "" },
                    ""
                ));
                styled_slots.insert(bank.slot());
            }
            first = false;

            match bank.parent() {
                None => {
                    if bank.slot() > 0 {
                        dot.push(format!(r#"    "{}" -> "..." [dir=back]"#, bank.slot(),));
                    }
                    break;
                }
                Some(parent) => {
                    let slot_distance = bank.slot() - parent.slot();
                    let penwidth = if bank.epoch() > parent.epoch() {
                        "5"
                    } else {
                        "1"
                    };
                    let link_label = if slot_distance > 1 {
                        format!("label=\"{} slots\",color=red", slot_distance)
                    } else {
                        "color=blue".to_string()
                    };
                    dot.push(format!(
                        r#"    "{}" -> "{}"[{},dir=back,penwidth={}];"#,
                        bank.slot(),
                        parent.slot(),
                        link_label,
                        penwidth
                    ));

                    bank = parent.clone();
                }
            }
        }
    }
    dot.push("  }".to_string());

    // Strafe the banks with links from validators to the bank they last voted on,
    // while collecting information about the absent votes and stakes
    let mut absent_stake = 0;
    let mut absent_votes = 0;
    let mut lowest_last_vote_slot = std::u64::MAX;
    let mut lowest_total_stake = 0;
    for (node_pubkey, (last_vote_slot, vote_state, stake, total_stake)) in &last_votes {
        all_votes.entry(*node_pubkey).and_modify(|validator_votes| {
            validator_votes.remove(&last_vote_slot);
        });

        dot.push(format!(
            r#"  "last vote {}"[shape=box,label="Latest validator vote: {}\nstake: {} SOL\nroot slot: {}\nvote history:\n{}"];"#,
            node_pubkey,
            node_pubkey,
            lamports_to_sol(*stake),
            vote_state.root_slot.unwrap_or(0),
            vote_state
                .votes
                .iter()
                .map(|vote| format!("slot {} (conf={})", vote.slot, vote.confirmation_count))
                .collect::<Vec<_>>()
                .join("\n")
        ));

        dot.push(format!(
            r#"  "last vote {}" -> "{}" [style=dashed,label="latest vote"];"#,
            node_pubkey,
            if styled_slots.contains(&last_vote_slot) {
                last_vote_slot.to_string()
            } else {
                if *last_vote_slot < lowest_last_vote_slot {
                    lowest_last_vote_slot = *last_vote_slot;
                    lowest_total_stake = *total_stake;
                }
                absent_votes += 1;
                absent_stake += stake;

                "...".to_string()
            },
        ));
    }

    // Annotate the final "..." node with absent vote and stake information
    if absent_votes > 0 {
        dot.push(format!(
            r#"    "..."[label="...\nvotes: {}, stake: {:.1} SOL {:.1}%"];"#,
            absent_votes,
            lamports_to_sol(absent_stake),
            absent_stake as f64 / lowest_total_stake as f64 * 100.,
        ));
    }

    // Add for vote information from all banks.
    if include_all_votes {
        for (node_pubkey, validator_votes) in &all_votes {
            for (vote_slot, vote_state) in validator_votes {
                dot.push(format!(
                    r#"  "{} vote {}"[shape=box,style=dotted,label="validator vote: {}\nroot slot: {}\nvote history:\n{}"];"#,
                    node_pubkey,
                    vote_slot,
                    node_pubkey,
                    vote_state.root_slot.unwrap_or(0),
                    vote_state
                        .votes
                        .iter()
                        .map(|vote| format!("slot {} (conf={})", vote.slot, vote.confirmation_count))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));

                dot.push(format!(
                    r#"  "{} vote {}" -> "{}" [style=dotted,label="vote"];"#,
                    node_pubkey,
                    vote_slot,
                    if styled_slots.contains(&vote_slot) {
                        vote_slot.to_string()
                    } else {
                        "...".to_string()
                    },
                ));
            }
        }
    }

    dot.push("}".to_string());
    dot.join("\n")
}

fn analyze_column<
    T: solana_ledger::blockstore_db::Column + solana_ledger::blockstore_db::ColumnName,
>(
    db: &Database,
    name: &str,
    key_size: usize,
) -> Result<(), String> {
    let mut key_tot: u64 = 0;
    let mut val_hist = histogram::Histogram::new();
    let mut val_tot: u64 = 0;
    let mut row_hist = histogram::Histogram::new();
    let a = key_size as u64;
    for (_x, y) in db.iter::<T>(blockstore_db::IteratorMode::Start).unwrap() {
        let b = y.len() as u64;
        key_tot += a;
        val_hist.increment(b).unwrap();
        val_tot += b;
        row_hist.increment(a + b).unwrap();
    }

    let json_result = if val_hist.entries() > 0 {
        json!({
            "column":name,
            "entries":val_hist.entries(),
            "key_stats":{
                "max":a,
                "total_bytes":key_tot,
            },
            "val_stats":{
                "p50":val_hist.percentile(50.0).unwrap(),
                "p90":val_hist.percentile(90.0).unwrap(),
                "p99":val_hist.percentile(99.0).unwrap(),
                "p999":val_hist.percentile(99.9).unwrap(),
                "min":val_hist.minimum().unwrap(),
                "max":val_hist.maximum().unwrap(),
                "stddev":val_hist.stddev().unwrap(),
                "total_bytes":val_tot,
            },
            "row_stats":{
                "p50":row_hist.percentile(50.0).unwrap(),
                "p90":row_hist.percentile(90.0).unwrap(),
                "p99":row_hist.percentile(99.0).unwrap(),
                "p999":row_hist.percentile(99.9).unwrap(),
                "min":row_hist.minimum().unwrap(),
                "max":row_hist.maximum().unwrap(),
                "stddev":row_hist.stddev().unwrap(),
                "total_bytes":key_tot + val_tot,
            },
        })
    } else {
        json!({
        "column":name,
        "entries":val_hist.entries(),
        "key_stats":{
            "max":a,
            "total_bytes":0,
        },
        "val_stats":{
            "total_bytes":0,
        },
        "row_stats":{
            "total_bytes":0,
        },
        })
    };

    println!("{}", serde_json::to_string_pretty(&json_result).unwrap());

    Ok(())
}

fn analyze_storage(database: &Database) -> Result<(), String> {
    use blockstore_db::columns::*;
    analyze_column::<SlotMeta>(database, "SlotMeta", SlotMeta::key_size())?;
    analyze_column::<Orphans>(database, "Orphans", Orphans::key_size())?;
    analyze_column::<DeadSlots>(database, "DeadSlots", DeadSlots::key_size())?;
    analyze_column::<ErasureMeta>(database, "ErasureMeta", ErasureMeta::key_size())?;
    analyze_column::<Root>(database, "Root", Root::key_size())?;
    analyze_column::<Index>(database, "Index", Index::key_size())?;
    analyze_column::<ShredData>(database, "ShredData", ShredData::key_size())?;
    analyze_column::<ShredCode>(database, "ShredCode", ShredCode::key_size())?;
    analyze_column::<TransactionStatus>(
        database,
        "TransactionStatus",
        TransactionStatus::key_size(),
    )?;
    analyze_column::<TransactionStatus>(
        database,
        "TransactionStatusIndex",
        TransactionStatusIndex::key_size(),
    )?;
    analyze_column::<AddressSignatures>(
        database,
        "AddressSignatures",
        AddressSignatures::key_size(),
    )?;
    analyze_column::<Rewards>(database, "Rewards", Rewards::key_size())?;

    Ok(())
}

fn open_blockstore(ledger_path: &Path, access_type: AccessType) -> Blockstore {
    match Blockstore::open_with_access_type(ledger_path, access_type) {
        Ok(blockstore) => blockstore,
        Err(err) => {
            eprintln!("Failed to open ledger at {:?}: {:?}", ledger_path, err);
            exit(1);
        }
    }
}

fn open_database(ledger_path: &Path, access_type: AccessType) -> Database {
    match Database::open(&ledger_path.join("rocksdb"), access_type) {
        Ok(database) => database,
        Err(err) => {
            eprintln!("Unable to read the Ledger rocksdb: {:?}", err);
            exit(1);
        }
    }
}

// This function is duplicated in validator/src/main.rs...
fn hardforks_of(matches: &ArgMatches<'_>, name: &str) -> Option<Vec<Slot>> {
    if matches.is_present(name) {
        Some(values_t_or_exit!(matches, name, Slot))
    } else {
        None
    }
}

fn load_bank_forks(
    arg_matches: &ArgMatches,
    ledger_path: &PathBuf,
    genesis_config: &GenesisConfig,
    process_options: ProcessOptions,
    access_type: AccessType,
) -> bank_forks_utils::LoadResult {
    let blockstore = open_blockstore(&ledger_path, access_type);
    let snapshot_path = ledger_path.clone().join(if blockstore.is_primary_access() {
        "snapshot"
    } else {
        "snapshot.ledger-tool"
    });
    let snapshot_config = if arg_matches.is_present("no_snapshot") {
        None
    } else {
        Some(SnapshotConfig {
            snapshot_interval_slots: 0, // Value doesn't matter
            snapshot_package_output_path: ledger_path.clone(),
            snapshot_path,
            compression: CompressionType::Bzip2,
            snapshot_version: SnapshotVersion::default(),
        })
    };
    let account_paths = if let Some(account_paths) = arg_matches.value_of("account_paths") {
        if !blockstore.is_primary_access() {
            // Be defenstive, when default account dir is explicitly specified, it's still possible
            // to wipe the dir possibly shared by the running validator!
            eprintln!("Error: custom accounts path is not supported under secondary access");
            exit(1);
        }
        account_paths.split(',').map(PathBuf::from).collect()
    } else if blockstore.is_primary_access() {
        vec![ledger_path.join("accounts")]
    } else {
        let non_primary_accounts_path = ledger_path.join("accounts.ledger-tool");
        warn!(
            "Default accounts path is switched aligning with Blockstore's secondary access: {:?}",
            non_primary_accounts_path
        );
        vec![non_primary_accounts_path]
    };

    bank_forks_utils::load(
        &genesis_config,
        &blockstore,
        account_paths,
        snapshot_config.as_ref(),
        process_options,
    )
}

fn open_genesis_config_by(ledger_path: &Path, matches: &ArgMatches<'_>) -> GenesisConfig {
    let max_genesis_archive_unpacked_size =
        value_t_or_exit!(matches, "max_genesis_archive_unpacked_size", u64);
    open_genesis_config(ledger_path, max_genesis_archive_unpacked_size)
}

#[allow(clippy::cognitive_complexity)]
fn main() {
    // Ignore SIGUSR1 to prevent long-running calls being killed by logrotate
    // in warehouse deployments
    #[cfg(unix)]
    {
        // `register()` is unsafe because the action is called in a signal handler
        // with the usual caveats. So long as this action body stays empty, we'll
        // be fine
        unsafe { signal_hook::register(signal_hook::SIGUSR1, || {}) }.unwrap();
    }

    const DEFAULT_ROOT_COUNT: &str = "1";
    solana_logger::setup_with_default("solana=info");

    let starting_slot_arg = Arg::with_name("starting_slot")
        .long("starting-slot")
        .value_name("NUM")
        .takes_value(true)
        .default_value("0")
        .help("Start at this slot");
    let no_snapshot_arg = Arg::with_name("no_snapshot")
        .long("no-snapshot")
        .takes_value(false)
        .help("Do not start from a local snapshot if present");
    let account_paths_arg = Arg::with_name("account_paths")
        .long("accounts")
        .value_name("PATHS")
        .takes_value(true)
        .help("Comma separated persistent accounts location");
    let halt_at_slot_arg = Arg::with_name("halt_at_slot")
        .long("halt-at-slot")
        .value_name("SLOT")
        .validator(is_slot)
        .takes_value(true)
        .help("Halt processing at the given slot");
    let hard_forks_arg = Arg::with_name("hard_forks")
        .long("hard-fork")
        .value_name("SLOT")
        .validator(is_slot)
        .multiple(true)
        .takes_value(true)
        .help("Add a hard fork at this slot");
    let allow_dead_slots_arg = Arg::with_name("allow_dead_slots")
        .long("allow-dead-slots")
        .takes_value(false)
        .help("Output dead slots as well");
    let default_genesis_archive_unpacked_size = MAX_GENESIS_ARCHIVE_UNPACKED_SIZE.to_string();
    let max_genesis_archive_unpacked_size_arg = Arg::with_name("max_genesis_archive_unpacked_size")
        .long("max-genesis-archive-unpacked-size")
        .value_name("NUMBER")
        .takes_value(true)
        .default_value(&default_genesis_archive_unpacked_size)
        .help("maximum total uncompressed size of unpacked genesis archive");
    let snapshot_version_arg = Arg::with_name("snapshot_version")
        .long("snapshot-version")
        .value_name("SNAPSHOT_VERSION")
        .validator(is_parsable::<SnapshotVersion>)
        .takes_value(true)
        .default_value(SnapshotVersion::default().into())
        .help("Output snapshot version");
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(solana_version::version!())
        .arg(
            Arg::with_name("ledger_path")
                .short("l")
                .long("ledger")
                .value_name("DIR")
                .takes_value(true)
                .global(true)
                .help("Use DIR for ledger location"),
        )
        .subcommand(
            SubCommand::with_name("print")
            .about("Print the ledger")
            .arg(&starting_slot_arg)
            .arg(&allow_dead_slots_arg)
            .arg(
                Arg::with_name("num_slots")
                    .long("num-slots")
                    .value_name("SLOT")
                    .validator(is_slot)
                    .takes_value(true)
                    .help("Number of slots to print"),
            )
            .arg(
                Arg::with_name("only_rooted")
                    .long("only-rooted")
                    .takes_value(false)
                    .help("Only print root slots"),
            )
            .arg(
                Arg::with_name("verbose")
                    .long("verbose")
                    .short("v")
                    .multiple(true)
                    .takes_value(false)
                    .help("How verbose to print the ledger contents."),
            )
        )
        .subcommand(
            SubCommand::with_name("copy")
            .about("Copy the ledger")
            .arg(&starting_slot_arg)
            .arg(
                Arg::with_name("ending_slot")
                    .long("ending-slot")
                    .value_name("SLOT")
                    .validator(is_slot)
                    .takes_value(true)
                    .help("Slot to stop copy"),
            )
            .arg(
                Arg::with_name("target_db")
                    .long("target-db")
                    .value_name("PATH")
                    .takes_value(true)
                    .help("Target db"),
            )
        )
        .subcommand(
            SubCommand::with_name("slot")
            .about("Print the contents of one or more slots")
            .arg(
                Arg::with_name("slots")
                    .index(1)
                    .value_name("SLOTS")
                    .validator(is_slot)
                    .takes_value(true)
                    .multiple(true)
                    .required(true)
                    .help("Slots to print"),
            )
            .arg(&allow_dead_slots_arg)
        )
        .subcommand(
            SubCommand::with_name("set-dead-slot")
            .about("Mark one or more slots dead")
            .arg(
                Arg::with_name("slots")
                    .index(1)
                    .value_name("SLOTS")
                    .validator(is_slot)
                    .takes_value(true)
                    .multiple(true)
                    .required(true)
                    .help("Slots to mark dead"),
            )
        )
        .subcommand(
            SubCommand::with_name("genesis")
            .about("Prints the ledger's genesis config")
            .arg(&max_genesis_archive_unpacked_size_arg)
        )
        .subcommand(
            SubCommand::with_name("parse_full_frozen")
            .about("Parses log for information about critical events about ancestors of the given `ending_slot`")
            .arg(&starting_slot_arg)
            .arg(
                Arg::with_name("ending_slot")
                    .long("ending-slot")
                    .value_name("SLOT")
                    .takes_value(true)
                    .help("The last slot to iterate to"),
            )
            .arg(
                Arg::with_name("log_path")
                    .long("log-path")
                    .value_name("PATH")
                    .takes_value(true)
                    .help("path to log file to parse"),
            )
        )
        .subcommand(
            SubCommand::with_name("genesis-hash")
            .about("Prints the ledger's genesis hash")
            .arg(&max_genesis_archive_unpacked_size_arg)
        )
        .subcommand(
            SubCommand::with_name("shred-version")
            .about("Prints the ledger's shred hash")
            .arg(&hard_forks_arg)
            .arg(&max_genesis_archive_unpacked_size_arg)
        )
        .subcommand(
            SubCommand::with_name("bounds")
            .about("Print lowest and highest non-empty slots. Note that there may be empty slots within the bounds")
            .arg(
                Arg::with_name("all")
                    .long("all")
                    .takes_value(false)
                    .required(false)
                    .help("Additionally print all the non-empty slots within the bounds"),
            )
        ).subcommand(
            SubCommand::with_name("json")
            .about("Print the ledger in JSON format")
            .arg(&starting_slot_arg)
            .arg(&allow_dead_slots_arg)
        )
        .subcommand(
            SubCommand::with_name("verify")
            .about("Verify the ledger")
            .arg(&no_snapshot_arg)
            .arg(&account_paths_arg)
            .arg(&halt_at_slot_arg)
            .arg(&hard_forks_arg)
            .arg(&max_genesis_archive_unpacked_size_arg)
            .arg(
                Arg::with_name("skip_poh_verify")
                    .long("skip-poh-verify")
                    .takes_value(false)
                    .help("Skip ledger PoH verification"),
            )
        ).subcommand(
            SubCommand::with_name("graph")
            .about("Create a Graphviz rendering of the ledger")
            .arg(&no_snapshot_arg)
            .arg(&account_paths_arg)
            .arg(&halt_at_slot_arg)
            .arg(&hard_forks_arg)
            .arg(&max_genesis_archive_unpacked_size_arg)
            .arg(
                Arg::with_name("include_all_votes")
                    .long("include-all-votes")
                    .help("Include all votes in the graph"),
            )
            .arg(
                Arg::with_name("graph_filename")
                    .index(1)
                    .value_name("FILENAME")
                    .takes_value(true)
                    .help("Output file"),
            )
        ).subcommand(
            SubCommand::with_name("create-snapshot")
            .about("Create a new ledger snapshot")
            .arg(&no_snapshot_arg)
            .arg(&account_paths_arg)
            .arg(&hard_forks_arg)
            .arg(&max_genesis_archive_unpacked_size_arg)
            .arg(&snapshot_version_arg)
            .arg(
                Arg::with_name("snapshot_slot")
                    .index(1)
                    .value_name("SLOT")
                    .validator(is_slot)
                    .takes_value(true)
                    .help("Slot at which to create the snapshot"),
            )
            .arg(
                Arg::with_name("output_directory")
                    .index(2)
                    .value_name("DIR")
                    .takes_value(true)
                    .help("Output directory for the snapshot"),
            )
            .arg(
                Arg::with_name("warp_slot")
                    .required(false)
                    .long("warp-slot")
                    .takes_value(true)
                    .value_name("WARP_SLOT")
                    .validator(is_slot)
                    .help("After loading the snapshot slot warp the ledger to WARP_SLOT, \
                           which could be a slot in a galaxy far far away"),
            )

        ).subcommand(
            SubCommand::with_name("accounts")
            .about("Print account contents after processing in the ledger")
            .arg(&no_snapshot_arg)
            .arg(&account_paths_arg)
            .arg(&halt_at_slot_arg)
            .arg(&hard_forks_arg)
            .arg(
                Arg::with_name("include_sysvars")
                    .long("include-sysvars")
                    .takes_value(false)
                    .help("Include sysvars too"),
            )
            .arg(&max_genesis_archive_unpacked_size_arg)
        ).subcommand(
            SubCommand::with_name("capitalization")
            .about("Print capitalization (aka, total suppy)")
            .arg(&no_snapshot_arg)
            .arg(&account_paths_arg)
            .arg(&halt_at_slot_arg)
            .arg(&hard_forks_arg)
            .arg(&max_genesis_archive_unpacked_size_arg)
        ).subcommand(
            SubCommand::with_name("purge")
            .about("Delete a range of slots from the ledger.")
            .arg(
                Arg::with_name("start_slot")
                    .index(1)
                    .value_name("SLOT")
                    .takes_value(true)
                    .required(true)
                    .help("Start slot to purge from (inclusive)"),
            )
            .arg(
                Arg::with_name("end_slot")
                    .index(2)
                    .value_name("SLOT")
                    .required(true)
                    .help("Ending slot to stop purging (inclusive)"),
            )
        )
        .subcommand(
            SubCommand::with_name("list-roots")
            .about("Output upto last <num-roots> root hashes and their heights starting at the given block height")
            .arg(
                Arg::with_name("max_height")
                    .long("max-height")
                    .value_name("NUM")
                    .takes_value(true)
                    .required(true)
                    .help("Maximum block height")
            )
            .arg(
                Arg::with_name("slot_list")
                    .long("slot-list")
                    .value_name("FILENAME")
                    .required(false)
                    .takes_value(true)
                    .help("The location of the output YAML file. A list of rollback slot heights and hashes will be written to the file.")
            )
            .arg(
                Arg::with_name("num_roots")
                    .long("num-roots")
                    .value_name("NUM")
                    .takes_value(true)
                    .default_value(DEFAULT_ROOT_COUNT)
                    .required(false)
                    .help("Number of roots in the output"),
            )
        )
        .subcommand(
            SubCommand::with_name("analyze-storage")
                .about("Output statistics in JSON format about all column families in the ledger rocksDB")
        )
        .get_matches();

    let ledger_path = PathBuf::from(value_t!(matches, "ledger_path", String).unwrap_or_else(
        |_err| {
            eprintln!(
                "Error: Missing --ledger <DIR> argument.\n\n{}",
                matches.usage()
            );
            exit(1);
        },
    ));

    // Canonicalize ledger path to avoid issues with symlink creation
    let ledger_path = fs::canonicalize(&ledger_path).unwrap_or_else(|err| {
        eprintln!("Unable to access ledger path: {:?}", err);
        exit(1);
    });

    match matches.subcommand() {
        ("print", Some(arg_matches)) => {
            let starting_slot = value_t_or_exit!(arg_matches, "starting_slot", Slot);
            let num_slots = value_t!(arg_matches, "num_slots", Slot).ok();
            let allow_dead_slots = arg_matches.is_present("allow_dead_slots");
            let only_rooted = arg_matches.is_present("only_rooted");
            let verbose = arg_matches.occurrences_of("verbose");
            output_ledger(
                open_blockstore(&ledger_path, AccessType::TryPrimaryThenSecondary),
                starting_slot,
                allow_dead_slots,
                LedgerOutputMethod::Print,
                num_slots,
                verbose,
                only_rooted,
            );
        }
        ("copy", Some(arg_matches)) => {
            let starting_slot = value_t_or_exit!(arg_matches, "starting_slot", Slot);
            let ending_slot = value_t_or_exit!(arg_matches, "ending_slot", Slot);
            let target_db = PathBuf::from(value_t_or_exit!(arg_matches, "target_db", String));
            let source = open_blockstore(&ledger_path, AccessType::TryPrimaryThenSecondary);
            let target = open_blockstore(&target_db, AccessType::PrimaryOnly);
            for (slot, _meta) in source.slot_meta_iterator(starting_slot).unwrap() {
                if slot > ending_slot {
                    break;
                }
                if let Ok(shreds) = source.get_data_shreds_for_slot(slot, 0) {
                    if target.insert_shreds(shreds, None, true).is_err() {
                        warn!("error inserting shreds for slot {}", slot);
                    }
                }
            }
        }
        ("genesis", Some(arg_matches)) => {
            println!("{}", open_genesis_config_by(&ledger_path, arg_matches));
        }
        ("genesis-hash", Some(arg_matches)) => {
            println!(
                "{}",
                open_genesis_config_by(&ledger_path, arg_matches).hash()
            );
        }
        ("shred-version", Some(arg_matches)) => {
            let process_options = ProcessOptions {
                dev_halt_at_slot: Some(0),
                new_hard_forks: hardforks_of(arg_matches, "hard_forks"),
                poh_verify: false,
                ..ProcessOptions::default()
            };
            let genesis_config = open_genesis_config_by(&ledger_path, arg_matches);
            match load_bank_forks(
                arg_matches,
                &ledger_path,
                &genesis_config,
                process_options,
                AccessType::TryPrimaryThenSecondary,
            ) {
                Ok((bank_forks, _leader_schedule_cache, _snapshot_hash)) => {
                    println!(
                        "{}",
                        compute_shred_version(
                            &genesis_config.hash(),
                            Some(&bank_forks.working_bank().hard_forks().read().unwrap())
                        )
                    );
                }
                Err(err) => {
                    eprintln!("Failed to load ledger: {:?}", err);
                    exit(1);
                }
            }
        }
        ("slot", Some(arg_matches)) => {
            let slots = values_t_or_exit!(arg_matches, "slots", Slot);
            let allow_dead_slots = arg_matches.is_present("allow_dead_slots");
            let blockstore = open_blockstore(&ledger_path, AccessType::TryPrimaryThenSecondary);
            for slot in slots {
                println!("Slot {}", slot);
                if let Err(err) = output_slot(
                    &blockstore,
                    slot,
                    allow_dead_slots,
                    &LedgerOutputMethod::Print,
                    std::u64::MAX,
                ) {
                    eprintln!("{}", err);
                }
            }
        }
        ("json", Some(arg_matches)) => {
            let starting_slot = value_t_or_exit!(arg_matches, "starting_slot", Slot);
            let allow_dead_slots = arg_matches.is_present("allow_dead_slots");
            output_ledger(
                open_blockstore(&ledger_path, AccessType::TryPrimaryThenSecondary),
                starting_slot,
                allow_dead_slots,
                LedgerOutputMethod::Json,
                None,
                std::u64::MAX,
                true,
            );
        }
        ("set-dead-slot", Some(arg_matches)) => {
            let slots = values_t_or_exit!(arg_matches, "slots", Slot);
            let blockstore = open_blockstore(&ledger_path, AccessType::PrimaryOnly);
            for slot in slots {
                match blockstore.set_dead_slot(slot) {
                    Ok(_) => println!("Slot {} dead", slot),
                    Err(err) => eprintln!("Failed to set slot {} dead slot: {}", slot, err),
                }
            }
        }
        ("parse_full_frozen", Some(arg_matches)) => {
            let starting_slot = value_t_or_exit!(arg_matches, "starting_slot", Slot);
            let ending_slot = value_t_or_exit!(arg_matches, "ending_slot", Slot);
            let blockstore = open_blockstore(&ledger_path, AccessType::TryPrimaryThenSecondary);
            let mut ancestors = BTreeSet::new();
            if blockstore.meta(ending_slot).unwrap().is_none() {
                panic!("Ending slot doesn't exist");
            }
            for a in AncestorIterator::new(ending_slot, &blockstore) {
                ancestors.insert(a);
                if a <= starting_slot {
                    break;
                }
            }
            println!("ancestors: {:?}", ancestors.iter());

            let mut frozen = BTreeMap::new();
            let mut full = BTreeMap::new();
            let frozen_regex = Regex::new(r"bank frozen: (\d*)").unwrap();
            let full_regex = Regex::new(r"slot (\d*) is full").unwrap();

            let log_file = PathBuf::from(value_t_or_exit!(arg_matches, "log_path", String));
            let f = BufReader::new(File::open(log_file).unwrap());
            println!("Reading log file");
            for line in f.lines() {
                if let Ok(line) = line {
                    let parse_results = {
                        if let Some(slot_string) = frozen_regex.captures_iter(&line).next() {
                            Some((slot_string, &mut frozen))
                        } else if let Some(slot_string) = full_regex.captures_iter(&line).next() {
                            Some((slot_string, &mut full))
                        } else {
                            None
                        }
                    };

                    if let Some((slot_string, map)) = parse_results {
                        let slot = slot_string
                            .get(1)
                            .expect("Only one match group")
                            .as_str()
                            .parse::<u64>()
                            .unwrap();
                        if ancestors.contains(&slot) && !map.contains_key(&slot) {
                            map.insert(slot, line);
                        }
                        if slot == ending_slot
                            && frozen.contains_key(&slot)
                            && full.contains_key(&slot)
                        {
                            break;
                        }
                    }
                }
            }

            for ((slot1, frozen_log), (slot2, full_log)) in frozen.iter().zip(full.iter()) {
                assert_eq!(slot1, slot2);
                println!(
                    "Slot: {}\n, full: {}\n, frozen: {}",
                    slot1, full_log, frozen_log
                );
            }
        }
        ("verify", Some(arg_matches)) => {
            let process_options = ProcessOptions {
                dev_halt_at_slot: value_t!(arg_matches, "halt_at_slot", Slot).ok(),
                new_hard_forks: hardforks_of(arg_matches, "hard_forks"),
                poh_verify: !arg_matches.is_present("skip_poh_verify"),
                ..ProcessOptions::default()
            };
            println!(
                "genesis hash: {}",
                open_genesis_config_by(&ledger_path, arg_matches).hash()
            );

            load_bank_forks(
                arg_matches,
                &ledger_path,
                &open_genesis_config_by(&ledger_path, arg_matches),
                process_options,
                AccessType::TryPrimaryThenSecondary,
            )
            .unwrap_or_else(|err| {
                eprintln!("Ledger verification failed: {:?}", err);
                exit(1);
            });
            println!("Ok");
        }
        ("graph", Some(arg_matches)) => {
            let output_file = value_t_or_exit!(arg_matches, "graph_filename", String);

            let process_options = ProcessOptions {
                dev_halt_at_slot: value_t!(arg_matches, "halt_at_slot", Slot).ok(),
                new_hard_forks: hardforks_of(arg_matches, "hard_forks"),
                poh_verify: false,
                ..ProcessOptions::default()
            };

            match load_bank_forks(
                arg_matches,
                &ledger_path,
                &open_genesis_config_by(&ledger_path, arg_matches),
                process_options,
                AccessType::TryPrimaryThenSecondary,
            ) {
                Ok((bank_forks, _leader_schedule_cache, _snapshot_hash)) => {
                    let dot = graph_forks(&bank_forks, arg_matches.is_present("include_all_votes"));

                    let extension = Path::new(&output_file).extension();
                    let result = if extension == Some(OsStr::new("pdf")) {
                        render_dot(dot, &output_file, "pdf")
                    } else if extension == Some(OsStr::new("png")) {
                        render_dot(dot, &output_file, "png")
                    } else {
                        File::create(&output_file)
                            .and_then(|mut file| file.write_all(&dot.into_bytes()))
                    };

                    match result {
                        Ok(_) => println!("Wrote {}", output_file),
                        Err(err) => eprintln!("Unable to write {}: {}", output_file, err),
                    }
                }
                Err(err) => {
                    eprintln!("Failed to load ledger: {:?}", err);
                    exit(1);
                }
            }
        }
        ("create-snapshot", Some(arg_matches)) => {
            let snapshot_slot = value_t_or_exit!(arg_matches, "snapshot_slot", Slot);
            let output_directory = value_t_or_exit!(arg_matches, "output_directory", String);
            let warp_slot = value_t!(arg_matches, "warp_slot", Slot).ok();
            let snapshot_version =
                arg_matches
                    .value_of("snapshot_version")
                    .map_or(SnapshotVersion::default(), |s| {
                        s.parse::<SnapshotVersion>().unwrap_or_else(|e| {
                            eprintln!("Error: {}", e);
                            exit(1)
                        })
                    });
            let process_options = ProcessOptions {
                dev_halt_at_slot: Some(snapshot_slot),
                new_hard_forks: hardforks_of(arg_matches, "hard_forks"),
                poh_verify: false,
                ..ProcessOptions::default()
            };
            let genesis_config = open_genesis_config_by(&ledger_path, arg_matches);
            match load_bank_forks(
                arg_matches,
                &ledger_path,
                &genesis_config,
                process_options,
                AccessType::TryPrimaryThenSecondary,
            ) {
                Ok((bank_forks, _leader_schedule_cache, _snapshot_hash)) => {
                    let bank = bank_forks
                        .get(snapshot_slot)
                        .unwrap_or_else(|| {
                            eprintln!("Error: Slot {} is not available", snapshot_slot);
                            exit(1);
                        })
                        .clone();

                    let bank = if let Some(warp_slot) = warp_slot {
                        Arc::new(Bank::warp_from_parent(
                            &bank,
                            bank.collector_id(),
                            warp_slot,
                        ))
                    } else {
                        bank
                    };

                    println!(
                        "Creating a version {} snapshot of slot {}",
                        snapshot_version,
                        bank.slot(),
                    );
                    assert!(bank.is_complete());
                    bank.squash();
                    bank.clean_accounts();
                    bank.update_accounts_hash();

                    let temp_dir = tempfile::tempdir_in(ledger_path).unwrap_or_else(|err| {
                        eprintln!("Unable to create temporary directory: {}", err);
                        exit(1);
                    });

                    let storages: Vec<_> = bank.get_snapshot_storages();
                    snapshot_utils::add_snapshot(&temp_dir, &bank, &storages, snapshot_version)
                        .and_then(|slot_snapshot_paths| {
                            snapshot_utils::package_snapshot(
                                &bank,
                                &slot_snapshot_paths,
                                &temp_dir,
                                &bank.src.roots(),
                                output_directory,
                                storages,
                                CompressionType::Bzip2,
                                snapshot_version,
                            )
                        })
                        .and_then(|package| {
                            snapshot_utils::archive_snapshot_package(&package).map(|ok| {
                                println!(
                                    "Successfully created snapshot for slot {}, hash {}: {:?}",
                                    bank.slot(),
                                    bank.hash(),
                                    package.tar_output_file
                                );
                                println!(
                                    "Shred version: {}",
                                    compute_shred_version(
                                        &genesis_config.hash(),
                                        Some(&bank.hard_forks().read().unwrap())
                                    )
                                );
                                ok
                            })
                        })
                        .unwrap_or_else(|err| {
                            eprintln!("Unable to create snapshot archive: {}", err);
                            exit(1);
                        });
                }
                Err(err) => {
                    eprintln!("Failed to load ledger: {:?}", err);
                    exit(1);
                }
            }
        }
        ("accounts", Some(arg_matches)) => {
            let dev_halt_at_slot = value_t!(arg_matches, "halt_at_slot", Slot).ok();
            let process_options = ProcessOptions {
                dev_halt_at_slot,
                new_hard_forks: hardforks_of(arg_matches, "hard_forks"),
                poh_verify: false,
                ..ProcessOptions::default()
            };
            let genesis_config = open_genesis_config_by(&ledger_path, arg_matches);
            let include_sysvars = arg_matches.is_present("include_sysvars");
            match load_bank_forks(
                arg_matches,
                &ledger_path,
                &genesis_config,
                process_options,
                AccessType::TryPrimaryThenSecondary,
            ) {
                Ok((bank_forks, _leader_schedule_cache, _snapshot_hash)) => {
                    let slot = bank_forks.working_bank().slot();
                    let bank = bank_forks.get(slot).unwrap_or_else(|| {
                        eprintln!("Error: Slot {} is not available", slot);
                        exit(1);
                    });

                    let accounts: BTreeMap<_, _> = bank
                        .get_program_accounts(None)
                        .into_iter()
                        .filter(|(pubkey, _account)| {
                            include_sysvars || !solana_sdk::sysvar::is_sysvar_id(pubkey)
                        })
                        .collect();

                    println!("---");
                    for (pubkey, account) in accounts.into_iter() {
                        let data_len = account.data.len();
                        println!("{}:", pubkey);
                        println!("  - balance: {} SOL", lamports_to_sol(account.lamports));
                        println!("  - owner: '{}'", account.owner);
                        println!("  - executable: {}", account.executable);
                        println!("  - data: '{}'", bs58::encode(account.data).into_string());
                        println!("  - data_len: {}", data_len);
                    }
                }
                Err(err) => {
                    eprintln!("Failed to load ledger: {:?}", err);
                    exit(1);
                }
            }
        }
        ("capitalization", Some(arg_matches)) => {
            let dev_halt_at_slot = value_t!(arg_matches, "halt_at_slot", Slot).ok();
            let process_options = ProcessOptions {
                dev_halt_at_slot,
                new_hard_forks: hardforks_of(arg_matches, "hard_forks"),
                poh_verify: false,
                ..ProcessOptions::default()
            };
            let genesis_config = open_genesis_config_by(&ledger_path, arg_matches);
            match load_bank_forks(
                arg_matches,
                &ledger_path,
                &genesis_config,
                process_options,
                AccessType::TryPrimaryThenSecondary,
            ) {
                Ok((bank_forks, _leader_schedule_cache, _snapshot_hash)) => {
                    let slot = bank_forks.working_bank().slot();
                    let bank = bank_forks.get(slot).unwrap_or_else(|| {
                        eprintln!("Error: Slot {} is not available", slot);
                        exit(1);
                    });

                    use solana_sdk::native_token::LAMPORTS_PER_SOL;
                    use std::fmt::{Display, Formatter, Result};
                    pub struct Sol(u64);

                    impl Display for Sol {
                        fn fmt(&self, f: &mut Formatter) -> Result {
                            write!(
                                f,
                                "{}.{:09} SOL",
                                self.0 / LAMPORTS_PER_SOL,
                                self.0 % LAMPORTS_PER_SOL
                            )
                        }
                    }

                    let computed_capitalization: u64 = bank
                        .get_program_accounts(None)
                        .into_iter()
                        .filter_map(|(_pubkey, account)| {
                            if account.lamports == u64::max_value() {
                                return None;
                            }

                            let is_specially_retained =
                                solana_sdk::native_loader::check_id(&account.owner)
                                    || solana_sdk::sysvar::check_id(&account.owner);

                            if is_specially_retained {
                                // specially retained accounts are ensured to exist by
                                // alwaysing having a balance of 1 lamports, which is
                                // outside the capitalization calculation.
                                Some(account.lamports - 1)
                            } else {
                                Some(account.lamports)
                            }
                        })
                        .sum();

                    if bank.capitalization() != computed_capitalization {
                        panic!(
                            "Capitalization mismatch!?: {} != {}",
                            bank.capitalization(),
                            computed_capitalization
                        );
                    }
                    println!("Capitalization: {}", Sol(bank.capitalization()));
                }
                Err(err) => {
                    eprintln!("Failed to load ledger: {:?}", err);
                    exit(1);
                }
            }
        }
        ("purge", Some(arg_matches)) => {
            let start_slot = value_t_or_exit!(arg_matches, "start_slot", Slot);
            let end_slot = value_t_or_exit!(arg_matches, "end_slot", Slot);
            let blockstore = open_blockstore(&ledger_path, AccessType::PrimaryOnly);
            blockstore.purge_and_compact_slots(start_slot, end_slot);
            blockstore.purge_from_next_slots(start_slot, end_slot);
        }
        ("list-roots", Some(arg_matches)) => {
            let blockstore = open_blockstore(&ledger_path, AccessType::TryPrimaryThenSecondary);
            let max_height = if let Some(height) = arg_matches.value_of("max_height") {
                usize::from_str(height).expect("Maximum height must be a number")
            } else {
                panic!("Maximum height must be provided");
            };
            let num_roots = if let Some(roots) = arg_matches.value_of("num_roots") {
                usize::from_str(roots).expect("Number of roots must be a number")
            } else {
                usize::from_str(DEFAULT_ROOT_COUNT).unwrap()
            };

            let iter = RootedSlotIterator::new(0, &blockstore).expect("Failed to get rooted slot");

            let slot_hash: Vec<_> = iter
                .filter_map(|(slot, _meta)| {
                    if slot <= max_height as u64 {
                        let blockhash = blockstore
                            .get_slot_entries(slot, 0)
                            .unwrap()
                            .last()
                            .unwrap()
                            .hash;
                        Some((slot, blockhash))
                    } else {
                        None
                    }
                })
                .collect();

            let mut output_file: Box<dyn Write> =
                if let Some(path) = arg_matches.value_of("slot_list") {
                    match File::create(path) {
                        Ok(file) => Box::new(file),
                        _ => Box::new(stdout()),
                    }
                } else {
                    Box::new(stdout())
                };

            slot_hash
                .into_iter()
                .rev()
                .enumerate()
                .for_each(|(i, (slot, hash))| {
                    if i < num_roots {
                        output_file
                            .write_all(format!("{:?}: {:?}\n", slot, hash).as_bytes())
                            .expect("failed to write");
                    }
                });
        }
        ("bounds", Some(arg_matches)) => {
            match open_blockstore(&ledger_path, AccessType::TryPrimaryThenSecondary)
                .slot_meta_iterator(0)
            {
                Ok(metas) => {
                    let all = arg_matches.is_present("all");

                    let slots: Vec<_> = metas.map(|(slot, _)| slot).collect();
                    if slots.is_empty() {
                        println!("Ledger is empty");
                    } else {
                        let first = slots.first().unwrap();
                        let last = slots.last().unwrap_or_else(|| first);
                        if first != last {
                            println!("Ledger has data for slots {:?} to {:?}", first, last);
                            if all {
                                println!("Non-empty slots: {:?}", slots);
                            }
                        } else {
                            println!("Ledger has data for slot {:?}", first);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Unable to read the Ledger: {:?}", err);
                    exit(1);
                }
            }
        }
        ("analyze-storage", _) => {
            match analyze_storage(&open_database(
                &ledger_path,
                AccessType::TryPrimaryThenSecondary,
            )) {
                Ok(()) => {
                    println!("Ok.");
                }
                Err(err) => {
                    eprintln!("Unable to read the Ledger: {:?}", err);
                    exit(1);
                }
            }
        }
        ("", _) => {
            eprintln!("{}", matches.usage());
            exit(1);
        }
        _ => unreachable!(),
    };
}
