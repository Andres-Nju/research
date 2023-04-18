	pub fn import_verified_blocks(&self, io: &IoChannel<NetSyncMessage>) -> usize {
		let max_blocks_to_import = 64;

		let mut imported_blocks = Vec::with_capacity(max_blocks_to_import);
		let mut invalid_blocks = HashSet::new();
		let mut import_results = Vec::with_capacity(max_blocks_to_import);

		let _import_lock = self.import_lock.lock();
		let _timer = PerfTimer::new("import_verified_blocks");
		let blocks = self.block_queue.drain(max_blocks_to_import);

		let original_best = self.chain_info().best_block_hash;

		for block in blocks {
			let header = &block.header;

			if invalid_blocks.contains(&header.parent_hash) {
				invalid_blocks.insert(header.hash());
				continue;
			}
			let closed_block = self.check_and_close_block(&block);
			if let Err(_) = closed_block {
				invalid_blocks.insert(header.hash());
				continue;
			}
			imported_blocks.push(header.hash());

			// Are we committing an era?
			let ancient = if header.number() >= HISTORY {
				let n = header.number() - HISTORY;
				Some((n, self.chain.block_hash(n).unwrap()))
			} else {
				None
			};

			// Commit results
			let closed_block = closed_block.unwrap();
			let receipts = closed_block.block().receipts().clone();
			let traces = From::from(closed_block.block().traces().clone().unwrap_or_else(Vec::new));

			closed_block.drain()
				.commit(header.number(), &header.hash(), ancient)
				.expect("State DB commit failed.");

			// And update the chain after commit to prevent race conditions
			// (when something is in chain but you are not able to fetch details)
			let route = self.chain.insert_block(&block.bytes, receipts);
			self.tracedb.import(TraceImportRequest {
				traces: traces,
				block_hash: header.hash(),
				block_number: header.number(),
				enacted: route.enacted.clone(),
				retracted: route.retracted.len()
			});

			import_results.push(route);

			self.report.write().unwrap().accrue_block(&block);
			trace!(target: "client", "Imported #{} ({})", header.number(), header.hash());
		}

		let imported = imported_blocks.len();
		let invalid_blocks = invalid_blocks.into_iter().collect::<Vec<H256>>();

		{
			if !invalid_blocks.is_empty() {
				self.block_queue.mark_as_bad(&invalid_blocks);
			}
			if !imported_blocks.is_empty() {
				self.block_queue.mark_as_good(&imported_blocks);
			}
		}

		{
			if !imported_blocks.is_empty() && self.block_queue.queue_info().is_empty() {
				let (enacted, retracted) = self.calculate_enacted_retracted(import_results);

				if self.queue_info().is_empty() {
					self.miner.chain_new_blocks(self, &imported_blocks, &invalid_blocks, &enacted, &retracted);
				}

				io.send(NetworkIoMessage::User(SyncMessage::NewChainBlocks {
					imported: imported_blocks,
					invalid: invalid_blocks,
					enacted: enacted,
					retracted: retracted,
				})).unwrap();
			}
		}

		{
			if self.chain_info().best_block_hash != original_best {
				self.miner.update_sealing(self);
			}
		}

		imported
	}
