    pub fn put(&mut self, pubkey: &Pubkey, executor: Arc<dyn Executor>) {
        if !self.executors.contains_key(pubkey) {
            if self.executors.len() >= self.max {
                let mut least = u64::MAX;
                let default_key = Pubkey::default();
                let mut least_key = &default_key;
                for (key, (count, _)) in self.executors.iter() {
                    let count = count.load(Relaxed);
                    if count < least {
                        least = count;
                        least_key = key;
                    }
                }
                let least_key = *least_key;
                let _ = self.executors.remove(&least_key);
            }
            let _ = self
                .executors
                .insert(*pubkey, (AtomicU64::new(0), executor));
        }
    }
