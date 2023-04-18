fn test_validator_exit_2() {
    solana_logger::setup();
    error!("test_validator_exit_2");
    let num_nodes = 2;
    let mut validator_config = ValidatorConfig::default();
    validator_config.rpc_config.enable_validator_exit = true;
    validator_config.wait_for_supermajority = Some(0);

    let config = ClusterConfig {
        cluster_lamports: 10_000,
        node_stakes: vec![100; num_nodes],
        validator_configs: vec![validator_config.clone(); num_nodes],
        ..ClusterConfig::default()
    };
    let local = LocalCluster::new(&config);
    cluster_tests::validator_exit(&local.entry_point_info, num_nodes);
}
