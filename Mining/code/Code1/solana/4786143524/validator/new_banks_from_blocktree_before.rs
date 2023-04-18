pub fn new_banks_from_blocktree(
    expected_genesis_blockhash: Option<Hash>,
    blocktree_path: &Path,
    account_paths: Option<String>,
    snapshot_config: Option<SnapshotConfig>,
    verify_ledger: bool,
    dev_halt_at_slot: Option<Slot>,
) -> (
    Hash,
    BankForks,
    Vec<BankForksInfo>,
    Blocktree,
    Receiver<bool>,
    CompletedSlotsReceiver,
    LeaderScheduleCache,
    PohConfig,
) {
    let genesis_block = GenesisBlock::load(blocktree_path).expect("Failed to load genesis block");
    let genesis_blockhash = genesis_block.hash();

    if let Some(expected_genesis_blockhash) = expected_genesis_blockhash {
        if genesis_blockhash != expected_genesis_blockhash {
            panic!(
                "Genesis blockhash mismatch: expected {} but local genesis blockhash is {}",
                expected_genesis_blockhash, genesis_blockhash,
            );
        }
    }

    adjust_ulimit_nofile();

    let (blocktree, ledger_signal_receiver, completed_slots_receiver) =
        Blocktree::open_with_signal(blocktree_path).expect("Failed to open ledger database");

    let (mut bank_forks, bank_forks_info, leader_schedule_cache) = get_bank_forks(
        &genesis_block,
        &blocktree,
        account_paths,
        snapshot_config.as_ref(),
        verify_ledger,
        dev_halt_at_slot,
    );

    if snapshot_config.is_some() {
        bank_forks.set_snapshot_config(snapshot_config.unwrap());
    }

    (
        genesis_blockhash,
        bank_forks,
        bank_forks_info,
        blocktree,
        ledger_signal_receiver,
        completed_slots_receiver,
        leader_schedule_cache,
        genesis_block.poh_config,
    )
}
