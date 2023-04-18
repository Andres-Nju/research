    pub fn new_validator(
        keypair: Keypair,
        bank: Bank,
        entry_height: u64,
        ledger_tail: &[Entry],
        node: TestNode,
        entry_point: &NodeInfo,
        exit: Arc<AtomicBool>,
        ledger_path: Option<&str>,
        _sigverify_disabled: bool,
    ) -> Self {
        let bank = Arc::new(bank);
        let mut thread_hdls = vec![];
        let rpu = Rpu::new(
            &bank,
            node.sockets.requests,
            node.sockets.respond,
            exit.clone(),
        );
        thread_hdls.extend(rpu.thread_hdls());

        let rpc_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), RPC_PORT);
        let rpc_service = JsonRpcService::new(bank.clone(), rpc_addr, exit.clone());
        thread_hdls.extend(rpc_service.thread_hdls());

        let blob_recycler = BlobRecycler::default();
        let window =
            window::new_window_from_entries(ledger_tail, entry_height, &node.data, &blob_recycler);

        let crdt = Arc::new(RwLock::new(Crdt::new(node.data).expect("Crdt::new")));
        crdt.write()
            .expect("'crdt' write lock before insert() in pub fn replicate")
            .insert(&entry_point);

        let ncp = Ncp::new(
            &crdt,
            window.clone(),
            ledger_path,
            node.sockets.gossip,
            node.sockets.gossip_send,
            exit.clone(),
        ).expect("Ncp::new");

        let tvu = Tvu::new(
            keypair,
            &bank,
            entry_height,
            crdt.clone(),
            window.clone(),
            node.sockets.replicate,
            node.sockets.repair,
            node.sockets.retransmit,
            ledger_path,
            exit.clone(),
        );
        thread_hdls.extend(tvu.thread_hdls());
        thread_hdls.extend(ncp.thread_hdls());
        Fullnode { exit, thread_hdls }
    }
