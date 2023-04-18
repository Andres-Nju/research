fn run_deadlock_detection_thread() {
	use std::thread;
	use std::time::Duration;
	use parking_lot::deadlock;

	info!("Starting deadlock detection thread.");
	// Create a background thread which checks for deadlocks every 10s
	thread::spawn(move || {
		loop {
			thread::sleep(Duration::from_secs(10));
			let deadlocks = deadlock::check_deadlock();
			if deadlocks.is_empty() {
				continue;
			}

			warn!("{} {} detected", deadlocks.len(), Style::new().bold().paint("deadlock(s)"));
			for (i, threads) in deadlocks.iter().enumerate() {
				warn!("{} #{}", Style::new().bold().paint("Deadlock"), i);
				for t in threads {
					warn!("Thread Id {:#?}", t.thread_id());
					warn!("{:#?}", t.backtrace());
				}
			}
		}
	});
}
