pub fn new_fullnode_for_tests() -> (Fullnode, ContactInfo, Keypair, String) {
    use crate::blocktree::create_new_tmp_ledger;
    use crate::cluster_info::Node;

    let node_keypair = Arc::new(Keypair::new());
    let node = Node::new_localhost_with_pubkey(&node_keypair.pubkey());
    let contact_info = node.info.clone();

    let (mut genesis_block, mint_keypair) =
        GenesisBlock::new_with_leader(10_000, &contact_info.id, 42);
    genesis_block
        .native_instruction_processors
        .push(("solana_budget_program".to_string(), solana_budget_api::id()));

    let (ledger_path, _blockhash) = create_new_tmp_ledger!(&genesis_block);

    let voting_keypair = Keypair::new();
    let node = Fullnode::new(
        node,
        &node_keypair,
        &ledger_path,
        &voting_keypair.pubkey(),
        voting_keypair,
        None,
        &FullnodeConfig::default(),
    );
    discover_nodes(&contact_info.gossip, 1).expect("Node startup failed");
    (node, contact_info, mint_keypair, ledger_path)
}
