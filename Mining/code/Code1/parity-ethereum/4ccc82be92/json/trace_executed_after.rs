	fn trace_executed(&mut self, gas_used: U256, stack_push: &[U256], mem_diff: Option<(usize, &[u8])>, store_diff: Option<(U256, U256)>) {
		let info = ::evm::INSTRUCTIONS[self.instruction as usize];

		println!(
			"{{\"pc\":{pc},\"op\":{op},\"opName\":\"{name}\",\"gas\":{gas},\"gasCost\":{gas_cost},\"memory\":{memory},\"stack\":{stack},\"storage\":{storage},\"depth\":{depth}}}",
			pc = self.pc,
			op = self.instruction,
			name = info.name,
			gas = display::u256_as_str(&(gas_used + self.gas_cost)),
			gas_cost = display::u256_as_str(&self.gas_cost),
			memory = self.memory(),
			stack = self.stack(),
			storage = self.storage(),
			depth = self.depth,
		);

		self.gas_used = gas_used;

		let len = self.stack.len();
		self.stack.truncate(len - info.args);
		self.stack.extend_from_slice(stack_push);

		if let Some((pos, data)) = mem_diff {
			if self.memory.len() < (pos + data.len()) {
				self.memory.resize(pos + data.len(), 0);
			}
			self.memory[pos..pos + data.len()].copy_from_slice(data);
		}

		if let Some((pos, val)) = store_diff {
			self.storage.insert(pos.into(), val.into());
		}
	}
