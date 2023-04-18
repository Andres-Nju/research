    pub fn new(
        node: TestNode,
        leader: bool,
        ledger: LedgerFile,
        keypair: KeyPair,
        network_entry_for_validator: Option<SocketAddr>,
    ) -> FullNode {
        FullNode::new_internal(
            node,
            leader,
            ledger,
            keypair,
            network_entry_for_validator,
            false,
        )
    }
