    pub fn new(
        node: TestNode,
        leader: bool,
        ledger: LedgerFile,
        keypair_for_validator: Option<KeyPair>,
        network_entry_for_validator: Option<SocketAddr>,
    ) -> FullNode {
        FullNode::new_internal(
            node,
            leader,
            ledger,
            keypair_for_validator,
            network_entry_for_validator,
            false,
        )
    }
