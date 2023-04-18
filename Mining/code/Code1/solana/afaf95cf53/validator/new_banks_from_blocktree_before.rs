pub fn new_banks_from_blocktree(
    blocktree_path: &Path,
    account_paths: Option<String>,
    snapshot_config: Option<SnapshotConfig>,
    verify_ledger: bool,
    dev_halt_at_slot: Option<Slot>,
) -> (
    BankForks,
    Vec<BankForksInfo>,
    Blocktree,
    Receiver<bool>,
    CompletedSlotsReceiver,
    LeaderScheduleCache,
    PohConfig,
) {
    let genesis_block =
        GenesisBlock::load(blocktree_path).expect("Expected to successfully open genesis block");

    let (blocktree, ledger_signal_receiver, completed_slots_receiver) =
        Blocktree::open_with_signal(blocktree_path)
            .expect("Expected to successfully open database ledger");

    let (bank_forks, bank_forks_info, leader_schedule_cache) = get_bank_forks(
        &genesis_block,
        &blocktree,
        account_paths,
        snapshot_config,
        verify_ledger,
        dev_halt_at_slot,
    );

    (
        bank_forks,
        bank_forks_info,
        blocktree,
        ledger_signal_receiver,
        completed_slots_receiver,
        leader_schedule_cache,
        genesis_block.poh_config,
    )
}
