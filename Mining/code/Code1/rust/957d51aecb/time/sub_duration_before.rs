        pub fn sub_duration(&self, other: &Duration) -> Instant {
            Instant {
                t: self.t.checked_sub(dur2intervals(other))
                       .expect("overflow when adding duration to instant"),
            }
        }
