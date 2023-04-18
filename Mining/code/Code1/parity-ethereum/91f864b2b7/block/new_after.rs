	/// Create a new `OpenBlock` ready for transaction pushing.
	pub fn new(
		engine: &'x Engine,
		factories: Factories,
		tracing: bool,
		db: StateDB,
		parent: &Header,
		last_hashes: Arc<LastHashes>,
		author: Address,
		gas_range_target: (U256, U256),
		extra_data: Bytes,
	) -> Result<Self, Error> {
		let state = State::from_existing(db, parent.state_root().clone(), engine.account_start_nonce(), factories)?;
		let mut r = OpenBlock {
			block: ExecutedBlock::new(state, tracing),
			engine: engine,
			last_hashes: last_hashes,
		};

		r.block.base.header.set_parent_hash(parent.hash());
		r.block.base.header.set_number(parent.number() + 1);
		r.block.base.header.set_author(author);
		r.block.base.header.set_timestamp_now(parent.timestamp());
		r.block.base.header.set_extra_data(extra_data);
		r.block.base.header.note_dirty();

		let gas_floor_target = cmp::max(gas_range_target.0, engine.params().min_gas_limit);
		let gas_ceil_target = cmp::max(gas_range_target.1, gas_floor_target);
		engine.populate_from_parent(&mut r.block.base.header, parent, gas_floor_target, gas_ceil_target);
		engine.on_new_block(&mut r.block);
		Ok(r)
	}

	/// Alter the author for the block.
	pub fn set_author(&mut self, author: Address) { self.block.base.header.set_author(author); }

	/// Alter the timestamp of the block.
	pub fn set_timestamp(&mut self, timestamp: u64) { self.block.base.header.set_timestamp(timestamp); }

	/// Alter the difficulty for the block.
	pub fn set_difficulty(&mut self, a: U256) { self.block.base.header.set_difficulty(a); }

	/// Alter the gas limit for the block.
	pub fn set_gas_limit(&mut self, a: U256) { self.block.base.header.set_gas_limit(a); }

	/// Alter the gas limit for the block.
	pub fn set_gas_used(&mut self, a: U256) { self.block.base.header.set_gas_used(a); }

	/// Alter the uncles hash the block.
	pub fn set_uncles_hash(&mut self, h: H256) { self.block.base.header.set_uncles_hash(h); }

	/// Alter transactions root for the block.
	pub fn set_transactions_root(&mut self, h: H256) { self.block.base.header.set_transactions_root(h); }

	/// Alter the receipts root for the block.
	pub fn set_receipts_root(&mut self, h: H256) { self.block.base.header.set_receipts_root(h); }

	/// Alter the extra_data for the block.
	pub fn set_extra_data(&mut self, extra_data: Bytes) -> Result<(), BlockError> {
		if extra_data.len() > self.engine.maximum_extra_data_size() {
			Err(BlockError::ExtraDataOutOfBounds(OutOfBounds{min: None, max: Some(self.engine.maximum_extra_data_size()), found: extra_data.len()}))
		} else {
			self.block.base.header.set_extra_data(extra_data);
			Ok(())
		}
	}

	/// Add an uncle to the block, if possible.
	///
	/// NOTE Will check chain constraints and the uncle number but will NOT check
	/// that the header itself is actually valid.
	pub fn push_uncle(&mut self, valid_uncle_header: Header) -> Result<(), BlockError> {
		if self.block.base.uncles.len() + 1 > self.engine.maximum_uncle_count() {
			return Err(BlockError::TooManyUncles(OutOfBounds{min: None, max: Some(self.engine.maximum_uncle_count()), found: self.block.base.uncles.len() + 1}));
		}
		// TODO: check number
		// TODO: check not a direct ancestor (use last_hashes for that)
		self.block.base.uncles.push(valid_uncle_header);
		Ok(())
	}

	/// Get the environment info concerning this block.
	pub fn env_info(&self) -> EnvInfo {
		// TODO: memoise.
		EnvInfo {
			number: self.block.base.header.number(),
			author: self.block.base.header.author().clone(),
			timestamp: self.block.base.header.timestamp(),
			difficulty: self.block.base.header.difficulty().clone(),
			last_hashes: self.last_hashes.clone(),
			gas_used: self.block.receipts.last().map_or(U256::zero(), |r| r.gas_used),
			gas_limit: self.block.base.header.gas_limit().clone(),
		}
	}

	/// Push a transaction into the block.
	///
	/// If valid, it will be executed, and archived together with the receipt.
	pub fn push_transaction(&mut self, t: SignedTransaction, h: Option<H256>) -> Result<&Receipt, Error> {
		if self.block.transactions_set.contains(&t.hash()) {
			return Err(From::from(TransactionError::AlreadyImported));
		}

		let env_info = self.env_info();
//		info!("env_info says gas_used={}", env_info.gas_used);
		match self.block.state.apply(&env_info, self.engine, &t, self.block.traces.is_some()) {
			Ok(outcome) => {
				self.block.transactions_set.insert(h.unwrap_or_else(||t.hash()));
				self.block.base.transactions.push(t);
				let t = outcome.trace;
				self.block.traces.as_mut().map(|traces| traces.push(t));
				self.block.receipts.push(outcome.receipt);
				Ok(self.block.receipts.last().expect("receipt just pushed; qed"))
			}
			Err(x) => Err(From::from(x))
		}
	}

	/// Turn this into a `ClosedBlock`.
	pub fn close(self) -> ClosedBlock {
		let mut s = self;

		let unclosed_state = s.block.state.clone();

		s.engine.on_close_block(&mut s.block);
		s.block.base.header.set_transactions_root(ordered_trie_root(s.block.base.transactions.iter().map(|e| e.rlp_bytes().to_vec())));
		let uncle_bytes = s.block.base.uncles.iter().fold(RlpStream::new_list(s.block.base.uncles.len()), |mut s, u| {s.append_raw(&u.rlp(Seal::With), 1); s} ).out();
		s.block.base.header.set_uncles_hash(uncle_bytes.sha3());
		s.block.base.header.set_state_root(s.block.state.root().clone());
		s.block.base.header.set_receipts_root(ordered_trie_root(s.block.receipts.iter().map(|r| r.rlp_bytes().to_vec())));
		s.block.base.header.set_log_bloom(s.block.receipts.iter().fold(LogBloom::zero(), |mut b, r| {b = &b | &r.log_bloom; b})); //TODO: use |= operator
		s.block.base.header.set_gas_used(s.block.receipts.last().map_or(U256::zero(), |r| r.gas_used));

		ClosedBlock {
			block: s.block,
			uncle_bytes: uncle_bytes,
			last_hashes: s.last_hashes,
			unclosed_state: unclosed_state,
		}
	}

	/// Turn this into a `LockedBlock`.
	pub fn close_and_lock(self) -> LockedBlock {
		let mut s = self;

		s.engine.on_close_block(&mut s.block);
		if s.block.base.header.transactions_root().is_zero() || s.block.base.header.transactions_root() == &SHA3_NULL_RLP {
			s.block.base.header.set_transactions_root(ordered_trie_root(s.block.base.transactions.iter().map(|e| e.rlp_bytes().to_vec())));
		}
		let uncle_bytes = s.block.base.uncles.iter().fold(RlpStream::new_list(s.block.base.uncles.len()), |mut s, u| {s.append_raw(&u.rlp(Seal::With), 1); s} ).out();
		if s.block.base.header.uncles_hash().is_zero() {
			s.block.base.header.set_uncles_hash(uncle_bytes.sha3());
		}
		if s.block.base.header.receipts_root().is_zero() || s.block.base.header.receipts_root() == &SHA3_NULL_RLP {
			s.block.base.header.set_receipts_root(ordered_trie_root(s.block.receipts.iter().map(|r| r.rlp_bytes().to_vec())));
		}

		s.block.base.header.set_state_root(s.block.state.root().clone());
		s.block.base.header.set_log_bloom(s.block.receipts.iter().fold(LogBloom::zero(), |mut b, r| {b = &b | &r.log_bloom; b})); //TODO: use |= operator
		s.block.base.header.set_gas_used(s.block.receipts.last().map_or(U256::zero(), |r| r.gas_used));

		LockedBlock {
			block: s.block,
			uncle_bytes: uncle_bytes,
		}
	}

	#[cfg(test)]
	/// Return mutable block reference. To be used in tests only.
	pub fn block_mut (&mut self) -> &mut ExecutedBlock { &mut self.block }
}

impl<'x> IsBlock for OpenBlock<'x> {
	fn block(&self) -> &ExecutedBlock { &self.block }
}

impl<'x> IsBlock for ClosedBlock {
	fn block(&self) -> &ExecutedBlock { &self.block }
}

impl<'x> IsBlock for LockedBlock {
	fn block(&self) -> &ExecutedBlock { &self.block }
}

impl ClosedBlock {
	/// Get the hash of the header without seal arguments.
	pub fn hash(&self) -> H256 { self.header().rlp_sha3(Seal::Without) }

	/// Turn this into a `LockedBlock`, unable to be reopened again.
	pub fn lock(self) -> LockedBlock {
		LockedBlock {
			block: self.block,
			uncle_bytes: self.uncle_bytes,
		}
	}

	/// Given an engine reference, reopen the `ClosedBlock` into an `OpenBlock`.
	pub fn reopen(self, engine: &Engine) -> OpenBlock {
		// revert rewards (i.e. set state back at last transaction's state).
		let mut block = self.block;
		block.state = self.unclosed_state;
		OpenBlock {
			block: block,
			engine: engine,
