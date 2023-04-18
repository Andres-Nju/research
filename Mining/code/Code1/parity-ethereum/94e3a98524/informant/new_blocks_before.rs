	fn new_blocks(&self, imported: Vec<H256>, _invalid: Vec<H256>, _enacted: Vec<H256>, _retracted: Vec<H256>, _sealed: Vec<H256>, duration: u64) {
		let mut last_import = self.last_import.lock();
		let sync_state = self.sync.as_ref().map(|s| s.status().state);
		let importing = is_major_importing(sync_state, self.client.queue_info());

		let ripe = Instant::now() > *last_import + Duration::from_secs(1) && !importing;
		let txs_imported = imported.iter()
			.take(imported.len() - if ripe {1} else {0})
			.filter_map(|h| self.client.block(BlockID::Hash(*h)))
			.map(|b| BlockView::new(&b).transactions_count())
			.sum();

		if ripe {
			if let Some(block) = imported.last().and_then(|h| self.client.block(BlockID::Hash(*h))) {
				let view = BlockView::new(&block);
				let header = view.header();
				let tx_count = view.transactions_count();
				let size = block.len();
				let (skipped, skipped_txs) = (self.skipped.load(AtomicOrdering::Relaxed) + imported.len() - 1, self.skipped.load(AtomicOrdering::Relaxed) + txs_imported);
				info!(target: "import", "Imported {} {} ({} txs, {} Mgas, {} ms, {} KiB){}",
					Colour::White.bold().paint(format!("#{}", header.number())),
					Colour::White.bold().paint(format!("{}", header.hash())),
					Colour::Yellow.bold().paint(format!("{}", tx_count)),
					Colour::Yellow.bold().paint(format!("{:.2}", header.gas_used().low_u64() as f32 / 1000000f32)),
					Colour::Purple.bold().paint(format!("{:.2}", duration as f32 / 1000000f32)),
					Colour::Blue.bold().paint(format!("{:.2}", size as f32 / 1024f32)),
					if skipped > 0 {
						format!(" + another {} block(s) containing {} tx(s)",
							Colour::Red.bold().paint(format!("{}", skipped)),
							Colour::Red.bold().paint(format!("{}", skipped_txs))
						)
					} else {
						String::new()
					}
				);
				self.skipped.store(0, AtomicOrdering::Relaxed);
				self.skipped_txs.store(0, AtomicOrdering::Relaxed);
				*last_import = Instant::now();
			}
		} else {
			self.skipped.fetch_add(imported.len(), AtomicOrdering::Relaxed);
			self.skipped_txs.fetch_add(txs_imported, AtomicOrdering::Relaxed);
		}
	}
