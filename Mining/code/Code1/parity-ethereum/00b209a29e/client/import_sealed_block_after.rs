	fn import_sealed_block(&self, block: SealedBlock) -> ImportResult {
		let h = block.header().hash();
		let start = Instant::now();
		let route = {
			// scope for self.import_lock
			let _import_lock = self.importer.import_lock.lock();
			trace_time!("import_sealed_block");

			let number = block.header().number();
			let block_data = block.rlp_bytes();
			let header = block.header().clone();

			let route = self.importer.commit_block(block, &header, &block_data, self);
			trace!(target: "client", "Imported sealed block #{} ({})", number, h);
			self.state_db.write().sync_cache(&route.enacted, &route.retracted, false);
			route
		};
		let route = ChainRoute::from([route].as_ref());
		self.importer.miner.chain_new_blocks(self, &[h.clone()], &[], route.enacted(), route.retracted(), self.engine.seals_internally().is_some());
		self.notify(|notify| {
			notify.new_blocks(
				vec![h.clone()],
				vec![],
				route.clone(),
				vec![h.clone()],
				vec![],
				start.elapsed(),
			);
		});
		self.db.read().flush().expect("DB flush failed.");
		Ok(h)
	}
