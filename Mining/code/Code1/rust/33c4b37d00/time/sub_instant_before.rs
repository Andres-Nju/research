    pub fn sub_instant(&self, other: &Instant) -> Duration {
        self.t.sub_timespec(&other.t).unwrap_or_else(|_| {
            panic!("other was less than the current instant")
        })
    }
