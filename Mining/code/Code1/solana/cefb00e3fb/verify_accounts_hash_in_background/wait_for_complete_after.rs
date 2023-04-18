    pub fn wait_for_complete(&self) {
        // just now completing
        let mut lock = self.thread.lock().unwrap();
        if lock.is_none() {
            return; // nothing to do
        }
        let result = lock.take().unwrap().join().unwrap();
        if !result {
            panic!("initial hash verification failed: {result:?}");
        }
        // we never have to check again
        self.verification_complete();
    }
