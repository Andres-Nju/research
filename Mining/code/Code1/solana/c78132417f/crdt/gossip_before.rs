    pub fn gossip(
        obj: Arc<RwLock<Self>>,
        blob_recycler: BlobRecycler,
        blob_sender: BlobSender,
        exit: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        let timeout = obj.read().unwrap().timeout.clone();
        Builder::new()
            .name("solana-gossip".to_string())
            .spawn(move || loop {
                let _ = Self::run_gossip(&obj, &blob_sender, &blob_recycler);
                if exit.load(Ordering::Relaxed) {
                    return;
                }
                //TODO this should be a tuned parameter
                sleep(timeout);
            })
            .unwrap()
    }
