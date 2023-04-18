fn test_two_unbalanced_stakes() {
    solana_logger::setup();
    let mut fullnode_config = FullnodeConfig::default();
    let num_ticks_per_second = 100;
    let num_ticks_per_slot = 40;
    let num_slots_per_epoch = MINIMUM_SLOT_LENGTH as u64;
    fullnode_config.tick_config =
        PohServiceConfig::Sleep(Duration::from_millis(1000 / num_ticks_per_second));
    fullnode_config.rpc_config.enable_fullnode_exit = true;
    let mut cluster = LocalCluster::new(&ClusterConfig {
        node_stakes: vec![999_990, 3],
        cluster_lamports: 1_000_000,
        fullnode_config: fullnode_config.clone(),
        ticks_per_slot: num_ticks_per_slot,
        slots_per_epoch: num_slots_per_epoch,
        ..ClusterConfig::default()
    });

    cluster_tests::sleep_n_epochs(
        10.0,
        &fullnode_config.tick_config,
        num_ticks_per_slot,
        num_slots_per_epoch,
    );
    cluster.close_preserve_ledgers();
    let leader_id = cluster.entry_point_info.id;
    let leader_ledger = cluster.fullnode_infos[&leader_id].ledger_path.clone();
    cluster_tests::verify_ledger_ticks(&leader_ledger, num_ticks_per_slot as usize);
}
