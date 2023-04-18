    pub fn finish_load(&mut self, load: &LoadType) {
        debug!("Removing blocking load {:?} ({}).", load, self.blocking_loads.len());
        let idx = self.blocking_loads.iter().position(|unfinished| *unfinished == *load);
        self.blocking_loads.remove(idx.expect(&format!("unknown completed load {:?}", load)));
    }
