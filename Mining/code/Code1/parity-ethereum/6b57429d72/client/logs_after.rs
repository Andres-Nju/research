	fn logs(&self, filter: Filter) -> Result<Vec<LocalizedLogEntry>, BlockId> {
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
			//
			// If we are sure the block does not exist (where val > best_block_number), then return error. Note that we
			// don't need to care about pending blocks here because RPC query sets pending back to latest (or handled
			// pending logs themselves).
			let from = match self.block_number_ref(&filter.from_block) {
				Some(val) if val <= chain.best_block_number() => val,
				_ => return Err(filter.from_block.clone()),
			};
			let to = match self.block_number_ref(&filter.to_block) {
				Some(val) if val <= chain.best_block_number() => val,
				_ => return Err(filter.to_block.clone()),
			};

			// If from is greater than to, then the current bloom filter behavior is to just return empty
			// result. There's no point to continue here.
			if from > to {
				return Err(filter.to_block.clone());
			}

			chain.blocks_with_bloom(&filter.bloom_possibilities(), from, to)
				.into_iter()
				.filter_map(|n| chain.block_hash(n))
				.collect::<Vec<H256>>()
		} else {
			// Otherwise, we use a slower version that finds a link between from_block and to_block.
			let from_hash = match Self::block_hash(&chain, filter.from_block) {
				Some(val) => val,
				None => return Err(filter.from_block.clone()),
			};
			let from_number = match chain.block_number(&from_hash) {
				Some(val) => val,
				None => return Err(BlockId::Hash(from_hash)),
			};
			let to_hash = match Self::block_hash(&chain, filter.to_block) {
				Some(val) => val,
				None => return Err(filter.to_block.clone()),
			};

			let blooms = filter.bloom_possibilities();
			let bloom_match = |header: &encoded::Header| {
				blooms.iter().any(|bloom| header.log_bloom().contains_bloom(bloom))
			};

			let (blocks, last_hash) = {
				let mut blocks = Vec::new();
				let mut current_hash = to_hash;

				loop {
					let header = match chain.block_header_data(&current_hash) {
						Some(val) => val,
						None => return Err(BlockId::Hash(current_hash)),
					};
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
				// In this case, from_hash is the cause (for not matching last_hash).
				return Err(BlockId::Hash(from_hash));
			}

			blocks
		};

		Ok(chain.logs(blocks, |entry| filter.matches(entry), filter.limit))
	}
