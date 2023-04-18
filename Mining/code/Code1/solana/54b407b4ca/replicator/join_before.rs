    pub fn join(self) {
        self.ncp.join().unwrap();
        self.t_window.join().unwrap();
        self.fetch_stage.join().unwrap();
        self.store_ledger_stage.join().unwrap();
    }
