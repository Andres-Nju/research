    pub fn leader_tpu_sockets(&self, fanout_slots: u64) -> Vec<SocketAddr> {
        self.leader_tpu_cache
            .read()
            .unwrap()
            .get_leader_sockets(fanout_slots)
    }
