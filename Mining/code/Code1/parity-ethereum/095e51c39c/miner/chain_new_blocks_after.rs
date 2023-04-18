	fn chain_new_blocks(&self, chain: &MiningBlockChainClient, _imported: &[H256], _invalid: &[H256], enacted: &[H256], retracted: &[H256]) {
		fn fetch_transactions(chain: &MiningBlockChainClient, hash: &H256) -> Vec<SignedTransaction> {
			let block = chain
				.block(BlockID::Hash(*hash))
				// Client should send message after commit to db and inserting to chain.
				.expect("Expected in-chain blocks.");
			let block = BlockView::new(&block);
			let txs = block.transactions();
			// populate sender
			for tx in &txs {
				let _sender = tx.sender();
			}
			txs
		}

		// 1. We ignore blocks that were `imported` (because it means that they are not in canon-chain, and transactions
		//	  should be still available in the queue.
		// 2. We ignore blocks that are `invalid` because it doesn't have any meaning in terms of the transactions that
		//    are in those blocks

		// First update gas limit in transaction queue
		self.update_gas_limit(chain);

		// Then import all transactions...
		{
			let out_of_chain = retracted
				.par_iter()
				.map(|h| fetch_transactions(chain, h));
			out_of_chain.for_each(|txs| {
				let mut transaction_queue = self.transaction_queue.lock().unwrap();
				let _ = self.add_transactions_to_queue(
					chain, txs, TransactionOrigin::External, &mut transaction_queue
				);
			});
		}

		// ...and at the end remove old ones
		{
			let in_chain = enacted
				.par_iter()
				.map(|h: &H256| fetch_transactions(chain, h));

			in_chain.for_each(|mut txs| {
				let mut transaction_queue = self.transaction_queue.lock().unwrap();

				let to_remove = txs.drain(..)
						.map(|tx| {
							tx.sender().expect("Transaction is in block, so sender has to be defined.")
						})
						.collect::<HashSet<Address>>();
				for sender in to_remove.into_iter() {
					transaction_queue.remove_all(sender, chain.latest_nonce(&sender));
				}
			});
		}

		self.update_sealing(chain);
	}
