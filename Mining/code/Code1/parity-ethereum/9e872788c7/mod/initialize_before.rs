	fn initialize(&self, io: &IoContext<()>) {
		let remaining = self.step.inner.duration_remaining().as_millis();
		io.register_timer_once(ENGINE_TIMEOUT_TOKEN, Duration::from_millis(remaining))
			.unwrap_or_else(|e| warn!(target: "engine", "Failed to start consensus step timer: {}.", e))
	}
