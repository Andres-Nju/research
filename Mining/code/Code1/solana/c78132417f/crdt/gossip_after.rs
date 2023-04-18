    pub fn gossip(
        obj: Arc<RwLock<Self>>,
        blob_recycler: BlobRecycler,
        blob_sender: BlobSender,
        exit: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        Builder::new()
            .name("solana-gossip".to_string())
            .spawn(move || loop {
                let _ = Self::run_gossip(&obj, &blob_sender, &blob_recycler);
                if exit.load(Ordering::Relaxed) {
                    return;
                }
                //TODO: possibly tune this parameter
                //we saw a deadlock passing an obj.read().unwrap().timeout into sleep
                sleep(Duration::from_millis(100));
            })
            .unwrap()
    }
