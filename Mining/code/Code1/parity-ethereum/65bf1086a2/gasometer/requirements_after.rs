	pub fn requirements(
		&mut self,
		ext: &vm::Ext,
		instruction: Instruction,
		info: &InstructionInfo,
		stack: &Stack<U256>,
		current_mem_size: usize,
	) -> vm::Result<InstructionRequirements<Gas>> {
		let schedule = ext.schedule();
		let tier = info.tier.idx();
		let default_gas = Gas::from(schedule.tier_step_gas[tier]);

		let cost = match instruction {
			instructions::JUMPDEST => {
				Request::Gas(Gas::from(1))
			},
			instructions::SSTORE => {
				let address = H256::from(stack.peek(0));
				let newval = stack.peek(1);
				let val = U256::from(&*ext.storage_at(&address)?);

				let gas = if schedule.eip1283 {
					let orig = U256::from(&*ext.initial_storage_at(&address)?);
					calculate_eip1283_sstore_gas(schedule, &orig, &val, &newval)
				} else {
					if val.is_zero() && !newval.is_zero() {
						schedule.sstore_set_gas
					} else {
						// Refund for below case is added when actually executing sstore
						// !is_zero(&val) && is_zero(newval)
						schedule.sstore_reset_gas
					}
				};
				Request::Gas(Gas::from(gas))
			},
			instructions::SLOAD => {
				Request::Gas(Gas::from(schedule.sload_gas))
			},
			instructions::BALANCE => {
				Request::Gas(Gas::from(schedule.balance_gas))
			},
			instructions::EXTCODESIZE => {
				Request::Gas(Gas::from(schedule.extcodesize_gas))
			},
			instructions::EXTCODEHASH => {
				Request::Gas(Gas::from(schedule.extcodehash_gas))
			},
			instructions::SUICIDE => {
				let mut gas = Gas::from(schedule.suicide_gas);

				let is_value_transfer = !ext.origin_balance()?.is_zero();
				let address = u256_to_address(stack.peek(0));
				if (
					!schedule.no_empty && !ext.exists(&address)?
				) || (
					schedule.no_empty && is_value_transfer && !ext.exists_and_not_null(&address)?
				) {
					gas = overflowing!(gas.overflow_add(schedule.suicide_to_new_account_cost.into()));
				}

				Request::Gas(gas)
			},
			instructions::MSTORE | instructions::MLOAD => {
				Request::GasMem(default_gas, mem_needed_const(stack.peek(0), 32)?)
			},
			instructions::MSTORE8 => {
				Request::GasMem(default_gas, mem_needed_const(stack.peek(0), 1)?)
			},
			instructions::RETURN | instructions::REVERT => {
				Request::GasMem(default_gas, mem_needed(stack.peek(0), stack.peek(1))?)
			},
			instructions::SHA3 => {
				let w = overflowing!(add_gas_usize(Gas::from_u256(*stack.peek(1))?, 31));
				let words = w >> 5;
				let gas = Gas::from(schedule.sha3_gas) + (Gas::from(schedule.sha3_word_gas) * words);
				Request::GasMem(gas, mem_needed(stack.peek(0), stack.peek(1))?)
			},
			instructions::CALLDATACOPY | instructions::CODECOPY | instructions::RETURNDATACOPY => {
				Request::GasMemCopy(default_gas, mem_needed(stack.peek(0), stack.peek(2))?, Gas::from_u256(*stack.peek(2))?)
			},
			instructions::EXTCODECOPY => {
				Request::GasMemCopy(schedule.extcodecopy_base_gas.into(), mem_needed(stack.peek(1), stack.peek(3))?, Gas::from_u256(*stack.peek(3))?)
			},
			instructions::LOG0 | instructions::LOG1 | instructions::LOG2 | instructions::LOG3 | instructions::LOG4 => {
				let no_of_topics = instruction.log_topics().expect("log_topics always return some for LOG* instructions; qed");
				let log_gas = schedule.log_gas + schedule.log_topic_gas * no_of_topics;

				let data_gas = overflowing!(Gas::from_u256(*stack.peek(1))?.overflow_mul(Gas::from(schedule.log_data_gas)));
				let gas = overflowing!(data_gas.overflow_add(Gas::from(log_gas)));
				Request::GasMem(gas, mem_needed(stack.peek(0), stack.peek(1))?)
			},
			instructions::CALL | instructions::CALLCODE => {
				let mut gas = Gas::from(schedule.call_gas);
				let mem = cmp::max(
					mem_needed(stack.peek(5), stack.peek(6))?,
					mem_needed(stack.peek(3), stack.peek(4))?
				);

				let address = u256_to_address(stack.peek(1));
				let is_value_transfer = !stack.peek(2).is_zero();

				if instruction == instructions::CALL && (
					(!schedule.no_empty && !ext.exists(&address)?)
					||
					(schedule.no_empty && is_value_transfer && !ext.exists_and_not_null(&address)?)
				) {
					gas = overflowing!(gas.overflow_add(schedule.call_new_account_gas.into()));
				}

				if is_value_transfer {
					gas = overflowing!(gas.overflow_add(schedule.call_value_transfer_gas.into()));
				}

				let requested = *stack.peek(0);

				Request::GasMemProvide(gas, mem, Some(requested))
			},
			instructions::DELEGATECALL | instructions::STATICCALL => {
				let gas = Gas::from(schedule.call_gas);
				let mem = cmp::max(
					mem_needed(stack.peek(4), stack.peek(5))?,
					mem_needed(stack.peek(2), stack.peek(3))?
				);
				let requested = *stack.peek(0);

				Request::GasMemProvide(gas, mem, Some(requested))
			},
			instructions::CREATE | instructions::CREATE2 => {
				let gas = Gas::from(schedule.create_gas);
				let mem = mem_needed(stack.peek(1), stack.peek(2))?;

				Request::GasMemProvide(gas, mem, None)
			},
			instructions::EXP => {
				let expon = stack.peek(1);
				let bytes = ((expon.bits() + 7) / 8) as usize;
				let gas = Gas::from(schedule.exp_gas + schedule.exp_byte_gas * bytes);
				Request::Gas(gas)
			},
			instructions::BLOCKHASH => {
				Request::Gas(Gas::from(schedule.blockhash_gas))
			},
			_ => Request::Gas(default_gas),
		};

		Ok(match cost {
			Request::Gas(gas) => {
				InstructionRequirements {
					gas_cost: gas,
					provide_gas: None,
					memory_required_size: 0,
					memory_total_gas: self.current_mem_gas,
				}
			},
			Request::GasMem(gas, mem_size) => {
				let (mem_gas_cost, new_mem_gas, new_mem_size) = self.mem_gas_cost(schedule, current_mem_size, &mem_size)?;
				let gas = overflowing!(gas.overflow_add(mem_gas_cost));
				InstructionRequirements {
					gas_cost: gas,
					provide_gas: None,
					memory_required_size: new_mem_size,
					memory_total_gas: new_mem_gas,
				}
			},
			Request::GasMemProvide(gas, mem_size, requested) => {
				let (mem_gas_cost, new_mem_gas, new_mem_size) = self.mem_gas_cost(schedule, current_mem_size, &mem_size)?;
				let gas = overflowing!(gas.overflow_add(mem_gas_cost));
				let provided = self.gas_provided(schedule, gas, requested)?;
				let total_gas = overflowing!(gas.overflow_add(provided));

				InstructionRequirements {
					gas_cost: total_gas,
					provide_gas: Some(provided),
					memory_required_size: new_mem_size,
					memory_total_gas: new_mem_gas,
				}
			},
			Request::GasMemCopy(gas, mem_size, copy) => {
				let (mem_gas_cost, new_mem_gas, new_mem_size) = self.mem_gas_cost(schedule, current_mem_size, &mem_size)?;
				let copy = overflowing!(add_gas_usize(copy, 31)) >> 5;
				let copy_gas = Gas::from(schedule.copy_gas) * copy;
				let gas = overflowing!(gas.overflow_add(copy_gas));
				let gas = overflowing!(gas.overflow_add(mem_gas_cost));

				InstructionRequirements {
					gas_cost: gas,
					provide_gas: None,
					memory_required_size: new_mem_size,
					memory_total_gas: new_mem_gas,
				}
			},
		})
	}
