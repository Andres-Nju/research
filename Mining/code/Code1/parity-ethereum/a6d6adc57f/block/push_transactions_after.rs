	fn push_transactions(&mut self, transactions: Vec<SignedTransaction>) -> Result<(), Error> {
		use std::time;

		let slow_tx = option_env!("SLOW_TX_DURATION").and_then(|v| v.parse().ok()).unwrap_or(100);
		for t in transactions {
			let hash = t.hash();
			let start = time::Instant::now();
			self.push_transaction(t, None)?;
			let took = start.elapsed();
			let took_ms = took.as_secs() * 1000 + took.subsec_nanos() as u64 / 1000000;
			if took > time::Duration::from_millis(slow_tx) {
				warn!("Heavy ({} ms) transaction in block {:?}: {:?}", took_ms, self.block.header().number(), hash);
			}
			debug!(target: "tx", "Transaction {:?} took: {} ms", hash, took_ms);
		}

		Ok(())
	}
