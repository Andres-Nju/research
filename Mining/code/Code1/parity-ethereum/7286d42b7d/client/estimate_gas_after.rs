	fn estimate_gas(&self, t: &SignedTransaction, block: BlockId) -> Result<U256, CallError> {
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
		let mut original_state = self.state_at(block).ok_or(CallError::StatePruned)?;
		let sender = t.sender().map_err(|e| {
			let message = format!("Transaction malformed: {:?}", e);
			ExecutionError::TransactionMalformed(message)
		})?;
		let balance = original_state.balance(&sender);
		let needed_balance = t.value + t.gas * t.gas_price;
		if balance < needed_balance {
			// give the sender a sufficient balance
			original_state.add_balance(&sender, &(needed_balance - balance), CleanupMode::NoEmpty);
		}
		let options = TransactOptions { tracing: true, vm_tracing: false, check_nonce: false };
		let mut tx = t.clone();

		let mut cond = |gas| {
			let mut state = original_state.clone();
			tx.gas = gas;
			Executive::new(&mut state, &env_info, &*self.engine, &self.factories.vm)
				.transact(&tx, options.clone())
				.map(|r| r.trace[0].result.succeeded())
				.unwrap_or(false)
		};

		let mut upper = env_info.gas_limit;
		if !cond(upper) {
			// impossible at block gas limit - try `UPPER_CEILING` instead.
			// TODO: consider raising limit by powers of two.
			const UPPER_CEILING: u64 = 1_000_000_000_000u64;
			upper = UPPER_CEILING.into();
			if !cond(upper) {
				trace!(target: "estimate_gas", "estimate_gas failed with {}", upper);
				return Err(CallError::Execution(ExecutionError::Internal))
			}
		}
		let lower = t.gas_required(&self.engine.schedule(&env_info)).into();
		if cond(lower) {
			trace!(target: "estimate_gas", "estimate_gas succeeded with {}", lower);
			return Ok(lower)
		}

		/// Find transition point between `lower` and `upper` where `cond` changes from `false` to `true`.
		/// Returns the lowest value between `lower` and `upper` for which `cond` returns true.
		/// We assert: `cond(lower) = false`, `cond(upper) = true`
		fn binary_chop<F>(mut lower: U256, mut upper: U256, mut cond: F) -> U256 where F: FnMut(U256) -> bool {
			while upper - lower > 1.into() {
				let mid = (lower + upper) / 2.into();
				trace!(target: "estimate_gas", "{} .. {} .. {}", lower, mid, upper);
				let c = cond(mid);
				match c {
					true => upper = mid,
					false => lower = mid,
				};
				trace!(target: "estimate_gas", "{} => {} .. {}", c, lower, upper);
			}
			upper
		}

		// binary chop to non-excepting call with gas somewhere between 21000 and block gas limit
		trace!(target: "estimate_gas", "estimate_gas chopping {} .. {}", lower, upper);
		Ok(binary_chop(lower, upper, cond))
	}
