    pub fn new(
        ledger_path: &Path,
        node: Node,
        cluster_entrypoint: ContactInfo,
        keypair: Arc<Keypair>,
        storage_keypair: Arc<Keypair>,
        client_commitment: CommitmentConfig,
    ) -> Result<Self> {
        let exit = Arc::new(AtomicBool::new(false));

        info!("Archiver: id: {}", keypair.pubkey());
        info!("Creating cluster info....");
        let mut cluster_info = ClusterInfo::new(node.info.clone(), keypair.clone());
        cluster_info.set_entrypoint(cluster_entrypoint.clone());
        let cluster_info = Arc::new(RwLock::new(cluster_info));
        let cluster_slots = Arc::new(ClusterSlots::default());
        // Note for now, this ledger will not contain any of the existing entries
        // in the ledger located at ledger_path, and will only append on newly received
        // entries after being passed to window_service
        let blockstore = Arc::new(
            Blockstore::open(ledger_path).expect("Expected to be able to open database ledger"),
        );

        let gossip_service = GossipService::new(&cluster_info, None, node.sockets.gossip, &exit);

        info!("Connecting to the cluster via {:?}", cluster_entrypoint);
        let (nodes, _) =
            match solana_core::gossip_service::discover_cluster(&cluster_entrypoint.gossip, 1) {
                Ok(nodes_and_archivers) => nodes_and_archivers,
                Err(e) => {
                    //shutdown services before exiting
                    exit.store(true, Ordering::Relaxed);
                    gossip_service.join()?;
                    return Err(e.into());
                }
            };
        let client = solana_core::gossip_service::get_client(&nodes);

        info!("Setting up mining account...");
        if let Err(e) =
            Self::setup_mining_account(&client, &keypair, &storage_keypair, client_commitment)
        {
            //shutdown services before exiting
            exit.store(true, Ordering::Relaxed);
            gossip_service.join()?;
            return Err(e);
        };

        let repair_socket = Arc::new(node.sockets.repair);
        let shred_sockets: Vec<Arc<UdpSocket>> =
            node.sockets.tvu.into_iter().map(Arc::new).collect();
        let shred_forward_sockets: Vec<Arc<UdpSocket>> = node
            .sockets
            .tvu_forwards
            .into_iter()
            .map(Arc::new)
            .collect();
        let (shred_fetch_sender, shred_fetch_receiver) = channel();
        let fetch_stage = ShredFetchStage::new(
            shred_sockets,
            shred_forward_sockets,
            repair_socket.clone(),
            &shred_fetch_sender,
            None,
            &exit,
        );
        let (slot_sender, slot_receiver) = channel();
        let request_processor =
            create_request_processor(node.sockets.storage.unwrap(), &exit, slot_receiver);

        let t_archiver = {
            let exit = exit.clone();
            let node_info = node.info.clone();
            let mut meta = ArchiverMeta {
                ledger_path: ledger_path.to_path_buf(),
                client_commitment,
                ..ArchiverMeta::default()
            };
            spawn(move || {
                // setup archiver
                let window_service = match Self::setup(
                    &mut meta,
                    cluster_info.clone(),
                    &blockstore,
                    &exit,
                    &node_info,
                    &storage_keypair,
                    repair_socket,
                    shred_fetch_receiver,
                    slot_sender,
                    cluster_slots,
                ) {
                    Ok(window_service) => window_service,
                    Err(e) => {
                        //shutdown services before exiting
                        error!("setup failed {:?}; archiver thread exiting...", e);
                        exit.store(true, Ordering::Relaxed);
                        request_processor
                            .into_iter()
                            .for_each(|t| t.join().unwrap());
                        fetch_stage.join().unwrap();
                        gossip_service.join().unwrap();
                        return;
                    }
                };

                info!("setup complete");
                // run archiver
                Self::run(
                    &mut meta,
                    &blockstore,
                    cluster_info,
                    &keypair,
                    &storage_keypair,
                    &exit,
                );
                // wait until exit
                request_processor
                    .into_iter()
                    .for_each(|t| t.join().unwrap());
                fetch_stage.join().unwrap();
                gossip_service.join().unwrap();
                window_service.join().unwrap()
            })
        };

        Ok(Self {
            thread_handles: vec![t_archiver],
            exit,
        })
    }
