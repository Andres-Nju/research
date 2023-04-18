	pub fn genesis_epoch_data(&self) -> Result<Vec<u8>, String> {
		use transaction::{Action, Transaction};
		use journaldb;
		use kvdb_memorydb;

		let genesis = self.genesis_header();

		let factories = Default::default();
		let mut db = journaldb::new(
			Arc::new(kvdb_memorydb::create(0)),
			journaldb::Algorithm::Archive,
			None,
		);

		self.ensure_db_good(BasicBackend(db.as_hashdb_mut()), &factories)
			.map_err(|e| format!("Unable to initialize genesis state: {}", e))?;

		let call = |a, d| {
			let mut db = db.boxed_clone();
			let env_info = ::evm::EnvInfo {
				number: 0,
				author: *genesis.author(),
				timestamp: genesis.timestamp(),
				difficulty: *genesis.difficulty(),
				gas_limit: *genesis.gas_limit(),
				last_hashes: Arc::new(Vec::new()),
				gas_used: 0.into(),
			};

			let from = Address::default();
			let tx = Transaction {
				nonce: self.engine.account_start_nonce(0),
				action: Action::Call(a),
				gas: U256::from(50_000_000), // TODO: share with client.
				gas_price: U256::default(),
				value: U256::default(),
				data: d,
			}.fake_sign(from);

			let res = ::state::prove_transaction(
				db.as_hashdb_mut(),
				*genesis.state_root(),
				&tx,
				self.engine.machine(),
				&env_info,
				factories.clone(),
				true,
			);

			res.map(|(out, proof)| {
				(out, proof.into_iter().map(|x| x.into_vec()).collect())
			}).ok_or_else(|| "Failed to prove call: insufficient state".into())
		};

		self.engine.genesis_epoch_data(&genesis, &call)
	}
