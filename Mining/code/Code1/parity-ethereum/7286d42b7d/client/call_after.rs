	fn call(&self, t: &SignedTransaction, block: BlockId, analytics: CallAnalytics) -> Result<Executed, CallError> {
		let header = self.block_header(block).ok_or(CallError::StatePruned)?;
		let last_hashes = self.build_last_hashes(header.parent_hash());
		let env_info = EnvInfo {
			number: header.number(),
			author: header.author(),
			timestamp: header.timestamp(),
			difficulty: header.difficulty(),
			last_hashes: last_hashes,
			gas_used: U256::zero(),
			gas_limit: U256::max_value(),
		};
		// that's just a copy of the state.
		let mut state = self.state_at(block).ok_or(CallError::StatePruned)?;
		let original_state = if analytics.state_diffing { Some(state.clone()) } else { None };

		let sender = t.sender().map_err(|e| {
			let message = format!("Transaction malformed: {:?}", e);
			ExecutionError::TransactionMalformed(message)
		})?;
		let balance = state.balance(&sender);
		let needed_balance = t.value + t.gas * t.gas_price;
		if balance < needed_balance {
			// give the sender a sufficient balance
			state.add_balance(&sender, &(needed_balance - balance), CleanupMode::NoEmpty);
		}
		let options = TransactOptions { tracing: analytics.transaction_tracing, vm_tracing: analytics.vm_tracing, check_nonce: false };
		let mut ret = Executive::new(&mut state, &env_info, &*self.engine, &self.factories.vm).transact(t, options)?;

		// TODO gav move this into Executive.
		ret.state_diff = original_state.map(|original| state.diff_from(original));

		Ok(ret)
	}
