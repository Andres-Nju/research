	fn replay(&self, id: TransactionId, analytics: CallAnalytics) -> Result<Executed, CallError> {
		let address = self.transaction_address(id).ok_or(CallError::TransactionNotFound)?;
		let block = BlockId::Hash(address.block_hash);

		const PROOF: &'static str = "The transaction address contains a valid index within block; qed";
		Ok(self.replay_block_transactions(block, analytics)?.nth(address.index).expect(PROOF).1)
	}

	fn replay_block_transactions(&self, block: BlockId, analytics: CallAnalytics) -> Result<Box<Iterator<Item = (H256, Executed)>>, CallError> {
		let mut env_info = self.env_info(block).ok_or(CallError::StatePruned)?;
		let body = self.block_body(block).ok_or(CallError::StatePruned)?;
		let mut state = self.state_at_beginning(block).ok_or(CallError::StatePruned)?;
		let txs = body.transactions();
		let engine = self.engine.clone();

		const PROOF: &'static str = "Transactions fetched from blockchain; blockchain transactions are valid; qed";
		const EXECUTE_PROOF: &'static str = "Transaction replayed; qed";

		Ok(Box::new(txs.into_iter()
			.map(move |t| {
				let transaction_hash = t.hash();
				let t = SignedTransaction::new(t).expect(PROOF);
				let machine = engine.machine();
				let x = Self::do_virtual_call(machine, &env_info, &mut state, &t, analytics).expect(EXECUTE_PROOF);
				env_info.gas_used = env_info.gas_used + x.gas_used;
				(transaction_hash, x)
			})))
	}

	fn mode(&self) -> Mode {
		let r = self.mode.lock().clone().into();
		trace!(target: "mode", "Asked for mode = {:?}. returning {:?}", &*self.mode.lock(), r);
		r
	}

	fn disable(&self) {
		self.set_mode(Mode::Off);
		self.enabled.store(false, AtomicOrdering::Relaxed);
		self.clear_queue();
	}

	fn set_mode(&self, new_mode: Mode) {
		trace!(target: "mode", "Client::set_mode({:?})", new_mode);
		if !self.enabled.load(AtomicOrdering::Relaxed) {
			return;
		}
		{
			let mut mode = self.mode.lock();
			*mode = new_mode.clone().into();
			trace!(target: "mode", "Mode now {:?}", &*mode);
			if let Some(ref mut f) = *self.on_user_defaults_change.lock() {
				trace!(target: "mode", "Making callback...");
				f(Some((&*mode).clone()))
			}
		}
		match new_mode {
			Mode::Active => self.wake_up(),
			Mode::Off => self.sleep(),
			_ => {(*self.sleep_state.lock()).last_activity = Some(Instant::now()); }
		}
	}

	fn spec_name(&self) -> String {
		self.config.spec_name.clone()
	}

	fn set_spec_name(&self, new_spec_name: String) {
		trace!(target: "mode", "Client::set_spec_name({:?})", new_spec_name);
		if !self.enabled.load(AtomicOrdering::Relaxed) {
			return;
		}
		if let Some(ref h) = *self.exit_handler.lock() {
			(*h)(new_spec_name);
		} else {
			warn!("Not hypervised; cannot change chain.");
		}
	}

	fn block_number(&self, id: BlockId) -> Option<BlockNumber> {
		self.block_number_ref(&id)
	}

	fn block_body(&self, id: BlockId) -> Option<encoded::Body> {
		let chain = self.chain.read();

		Self::block_hash(&chain, id).and_then(|hash| chain.block_body(&hash))
	}

	fn block_status(&self, id: BlockId) -> BlockStatus {
		let chain = self.chain.read();
		match Self::block_hash(&chain, id) {
			Some(ref hash) if chain.is_known(hash) => BlockStatus::InChain,
			Some(hash) => self.importer.block_queue.status(&hash).into(),
			None => BlockStatus::Unknown
		}
	}

	fn block_total_difficulty(&self, id: BlockId) -> Option<U256> {
		let chain = self.chain.read();

		Self::block_hash(&chain, id).and_then(|hash| chain.block_details(&hash)).map(|d| d.total_difficulty)
	}

	fn storage_root(&self, address: &Address, id: BlockId) -> Option<H256> {
		self.state_at(id).and_then(|s| s.storage_root(address).ok()).and_then(|x| x)
	}

	fn block_hash(&self, id: BlockId) -> Option<H256> {
		let chain = self.chain.read();
		Self::block_hash(&chain, id)
	}

	fn code(&self, address: &Address, state: StateOrBlock) -> Option<Option<Bytes>> {
		let result = match state {
			StateOrBlock::State(s) => s.code(address).ok(),
			StateOrBlock::Block(id) => self.state_at(id).and_then(|s| s.code(address).ok())
		};

		// Converting from `Option<Option<Arc<Bytes>>>` to `Option<Option<Bytes>>`
		result.map(|c| c.map(|c| (&*c).clone()))
	}

	fn storage_at(&self, address: &Address, position: &H256, state: StateOrBlock) -> Option<H256> {
		match state {
			StateOrBlock::State(s) => s.storage_at(address, position).ok(),
			StateOrBlock::Block(id) => self.state_at(id).and_then(|s| s.storage_at(address, position).ok())
		}
	}

	fn list_accounts(&self, id: BlockId, after: Option<&Address>, count: u64) -> Option<Vec<Address>> {
		if !self.factories.trie.is_fat() {
			trace!(target: "fatdb", "list_accounts: Not a fat DB");
			return None;
		}

		let state = match self.state_at(id) {
			Some(state) => state,
			_ => return None,
		};

		let (root, db) = state.drop();
		let trie = match self.factories.trie.readonly(db.as_hashdb(), &root) {
			Ok(trie) => trie,
			_ => {
				trace!(target: "fatdb", "list_accounts: Couldn't open the DB");
				return None;
			}
		};

		let mut iter = match trie.iter() {
			Ok(iter) => iter,
			_ => return None,
		};

		if let Some(after) = after {
			if let Err(e) = iter.seek(after) {
				trace!(target: "fatdb", "list_accounts: Couldn't seek the DB: {:?}", e);
			} else {
				// Position the iterator after the `after` element
				iter.next();
			}
		}

		let accounts = iter.filter_map(|item| {
			item.ok().map(|(addr, _)| Address::from_slice(&addr))
		}).take(count as usize).collect();

		Some(accounts)
	}

	fn list_storage(&self, id: BlockId, account: &Address, after: Option<&H256>, count: u64) -> Option<Vec<H256>> {
		if !self.factories.trie.is_fat() {
			trace!(target: "fatdb", "list_storage: Not a fat DB");
			return None;
		}

		let state = match self.state_at(id) {
			Some(state) => state,
			_ => return None,
		};

		let root = match state.storage_root(account) {
			Ok(Some(root)) => root,
			_ => return None,
		};

		let (_, db) = state.drop();
		let account_db = self.factories.accountdb.readonly(db.as_hashdb(), keccak(account));
		let trie = match self.factories.trie.readonly(account_db.as_hashdb(), &root) {
			Ok(trie) => trie,
			_ => {
				trace!(target: "fatdb", "list_storage: Couldn't open the DB");
				return None;
			}
		};

		let mut iter = match trie.iter() {
			Ok(iter) => iter,
			_ => return None,
		};

		if let Some(after) = after {
			if let Err(e) = iter.seek(after) {
				trace!(target: "fatdb", "list_storage: Couldn't seek the DB: {:?}", e);
			} else {
				// Position the iterator after the `after` element
				iter.next();
			}
		}

		let keys = iter.filter_map(|item| {
			item.ok().map(|(key, _)| H256::from_slice(&key))
		}).take(count as usize).collect();

		Some(keys)
	}

	fn transaction(&self, id: TransactionId) -> Option<LocalizedTransaction> {
		self.transaction_address(id).and_then(|address| self.chain.read().transaction(&address))
	}

	fn uncle(&self, id: UncleId) -> Option<encoded::Header> {
		let index = id.position;
		self.block_body(id.block).and_then(|body| body.view().uncle_rlp_at(index))
			.map(encoded::Header::new)
	}

	fn transaction_receipt(&self, id: TransactionId) -> Option<LocalizedReceipt> {
		let chain = self.chain.read();
		self.transaction_address(id)
			.and_then(|address| chain.block_number(&address.block_hash).and_then(|block_number| {
				let transaction = chain.block_body(&address.block_hash)
					.and_then(|body| body.view().localized_transaction_at(&address.block_hash, block_number, address.index));

				let previous_receipts = (0..address.index + 1)
					.map(|index| {
						let mut address = address.clone();
						address.index = index;
						chain.transaction_receipt(&address)
					})
					.collect();
				match (transaction, previous_receipts) {
					(Some(transaction), Some(previous_receipts)) => {
						Some(transaction_receipt(self.engine().machine(), transaction, previous_receipts))
					},
					_ => None,
				}
			}))
	}

	fn tree_route(&self, from: &H256, to: &H256) -> Option<TreeRoute> {
		let chain = self.chain.read();
		match chain.is_known(from) && chain.is_known(to) {
			true => chain.tree_route(from.clone(), to.clone()),
			false => None
		}
	}

	fn find_uncles(&self, hash: &H256) -> Option<Vec<H256>> {
		self.chain.read().find_uncle_hashes(hash, self.engine.maximum_uncle_age())
	}

	fn state_data(&self, hash: &H256) -> Option<Bytes> {
		self.state_db.read().journal_db().state(hash)
	}

	fn block_receipts(&self, hash: &H256) -> Option<Bytes> {
		self.chain.read().block_receipts(hash).map(|receipts| ::rlp::encode(&receipts).into_vec())
	}

	fn queue_info(&self) -> BlockQueueInfo {
		self.importer.block_queue.queue_info()
	}

	fn clear_queue(&self) {
		self.importer.block_queue.clear();
	}

	fn additional_params(&self) -> BTreeMap<String, String> {
		self.engine.additional_params().into_iter().collect()
	}

	fn logs(&self, filter: Filter) -> Vec<LocalizedLogEntry> {
		// Wrap the logic inside a closure so that we can take advantage of question mark syntax.
		let fetch_logs = || {
			let chain = self.chain.read();

			// First, check whether `filter.from_block` and `filter.to_block` is on the canon chain. If so, we can use the
			// optimized version.
			let is_canon = |id| {
				match id {
					// If it is referred by number, then it is always on the canon chain.
					&BlockId::Earliest | &BlockId::Latest | &BlockId::Number(_) => true,
					// If it is referred by hash, we see whether a hash -> number -> hash conversion gives us the same
					// result.
					&BlockId::Hash(ref hash) => chain.is_canon(hash),
				}
			};

			let blocks = if is_canon(&filter.from_block) && is_canon(&filter.to_block) {
				// If we are on the canon chain, use bloom filter to fetch required hashes.
				let from = self.block_number_ref(&filter.from_block)?;
				let to = self.block_number_ref(&filter.to_block)?;

				chain.blocks_with_bloom(&filter.bloom_possibilities(), from, to)
					.into_iter()
					.filter_map(|n| chain.block_hash(n))
					.collect::<Vec<H256>>()
			} else {
				// Otherwise, we use a slower version that finds a link between from_block and to_block.
				let from_hash = Self::block_hash(&chain, filter.from_block)?;
				let from_number = chain.block_number(&from_hash)?;
				let to_hash = Self::block_hash(&chain, filter.from_block)?;

				let blooms = filter.bloom_possibilities();
				let bloom_match = |header: &encoded::Header| {
					blooms.iter().any(|bloom| header.log_bloom().contains_bloom(bloom))
				};

				let (blocks, last_hash) = {
					let mut blocks = Vec::new();
					let mut current_hash = to_hash;

					loop {
						let header = chain.block_header_data(&current_hash)?;
						if bloom_match(&header) {
							blocks.push(current_hash);
						}

						// Stop if `from` block is reached.
						if header.number() <= from_number {
							break;
						}
						current_hash = header.parent_hash();
					}

					blocks.reverse();
					(blocks, current_hash)
				};

				// Check if we've actually reached the expected `from` block.
				if last_hash != from_hash || blocks.is_empty() {
					return None;
				}

				blocks
			};
