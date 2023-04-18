	fn pool_client<'a, C: 'a>(&'a self, chain: &'a C) -> PoolClient<'a, C> where
		C: BlockChain + CallContract,
	{
		PoolClient::new(
			chain,
			&self.nonce_cache,
			&*self.engine,
			self.accounts.as_ref().map(|x| &**x),
			self.options.refuse_service_transactions,
		)
	}

	/// Prepares new block for sealing including top transactions from queue.
	fn prepare_block<C>(&self, chain: &C) -> Option<(ClosedBlock, Option<H256>)> where
		C: BlockChain + CallContract + BlockProducer + Nonce + Sync,
	{
		trace_time!("prepare_block");
		let chain_info = chain.chain_info();

		// Open block
		let (mut open_block, original_work_hash) = {
			let mut sealing = self.sealing.lock();
			let last_work_hash = sealing.queue.peek_last_ref().map(|pb| pb.block().header().hash());
			let best_hash = chain_info.best_block_hash;

			// check to see if last ClosedBlock in would_seals is actually same parent block.
			// if so
			//   duplicate, re-open and push any new transactions.
			//   if at least one was pushed successfully, close and enqueue new ClosedBlock;
			//   otherwise, leave everything alone.
			// otherwise, author a fresh block.
			let mut open_block = match sealing.queue.pop_if(|b| b.block().header().parent_hash() == &best_hash) {
				Some(old_block) => {
					trace!(target: "miner", "prepare_block: Already have previous work; updating and returning");
					// add transactions to old_block
					chain.reopen_block(old_block)
				}
				None => {
					// block not found - create it.
					trace!(target: "miner", "prepare_block: No existing work - making new block");
					let params = self.params.read().clone();

					match chain.prepare_open_block(
						params.author,
						params.gas_range_target,
						params.extra_data,
					) {
						Ok(block) => block,
						Err(err) => {
							warn!(target: "miner", "Open new block failed with error {:?}. This is likely an error in chain specificiations or on-chain consensus smart contracts.", err);
							return None;
						}
					}
				}
			};

			if self.options.infinite_pending_block {
				open_block.remove_gas_limit();
			}

			(open_block, last_work_hash)
		};

		let mut invalid_transactions = HashSet::new();
		let mut not_allowed_transactions = HashSet::new();
		let mut senders_to_penalize = HashSet::new();
		let block_number = open_block.block().header().number();

		let mut tx_count = 0usize;
		let mut skipped_transactions = 0usize;

		let client = self.pool_client(chain);
		let engine_params = self.engine.params();
		let min_tx_gas: U256 = self.engine.schedule(chain_info.best_block_number).tx_gas.into();
		let nonce_cap: Option<U256> = if chain_info.best_block_number + 1 >= engine_params.dust_protection_transition {
			Some((engine_params.nonce_cap_increment * (chain_info.best_block_number + 1)).into())
		} else {
			None
		};
		// we will never need more transactions than limit divided by min gas
		let max_transactions = if min_tx_gas.is_zero() {
			usize::max_value()
		} else {
			MAX_SKIPPED_TRANSACTIONS.saturating_add(cmp::min(*open_block.block().header().gas_limit() / min_tx_gas, u64::max_value().into()).as_u64() as usize)
		};

		let pending: Vec<Arc<_>> = self.transaction_queue.pending(
			client.clone(),
			pool::PendingSettings {
				block_number: chain_info.best_block_number,
				current_timestamp: chain_info.best_block_timestamp,
				nonce_cap,
				max_len: max_transactions,
				ordering: miner::PendingOrdering::Priority,
			}
		);

		let took_ms = |elapsed: &Duration| {
			elapsed.as_secs() * 1000 + elapsed.subsec_nanos() as u64 / 1_000_000
		};

		let block_start = Instant::now();
		debug!(target: "miner", "Attempting to push {} transactions.", pending.len());

		for tx in pending {
			let start = Instant::now();

			let transaction = tx.signed().clone();
			let hash = transaction.hash();
			let sender = transaction.sender();

			// Re-verify transaction again vs current state.
			let result = client.verify_signed(&transaction)
				.map_err(|e| e.into())
				.and_then(|_| {
					open_block.push_transaction(transaction, None)
				});

			let took = start.elapsed();

			// Check for heavy transactions
			match self.options.tx_queue_penalization {
				Penalization::Enabled { ref offend_threshold } if &took > offend_threshold => {
					senders_to_penalize.insert(sender);
					debug!(target: "miner", "Detected heavy transaction ({} ms). Penalizing sender.", took_ms(&took));
				},
				_ => {},
			}

			debug!(target: "miner", "Adding tx {:?} took {} ms", hash, took_ms(&took));
			match result {
				Err(Error(ErrorKind::Execution(ExecutionError::BlockGasLimitReached { gas_limit, gas_used, gas }), _)) => {
					debug!(target: "miner", "Skipping adding transaction to block because of gas limit: {:?} (limit: {:?}, used: {:?}, gas: {:?})", hash, gas_limit, gas_used, gas);

					// Penalize transaction if it's above current gas limit
					if gas > gas_limit {
						debug!(target: "txqueue", "[{:?}] Transaction above block gas limit.", hash);
						invalid_transactions.insert(hash);
					}
