	fn timeout(&self, io: &IoContext<()>, timer: TimerToken) {
		if timer == ENGINE_TIMEOUT_TOKEN {
			// NOTE we might be lagging by couple of steps in case the timeout
			// has not been called fast enough.
			// Make sure to advance up to the actual step.
			while self.step.inner.duration_remaining().as_millis() == 0 {
				self.step.inner.increment();
				self.step.can_propose.store(true, AtomicOrdering::SeqCst);
				if let Some(ref weak) = *self.client.read() {
					if let Some(c) = weak.upgrade() {
						c.update_sealing();
					}
				}
			}

			let next_run_at = self.step.inner.duration_remaining().as_millis() >> 2;
			io.register_timer_once(ENGINE_TIMEOUT_TOKEN, Duration::from_millis(next_run_at))
				.unwrap_or_else(|e| warn!(target: "engine", "Failed to restart consensus step timer: {}.", e))
		}
	}
