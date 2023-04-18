	fn propagate_transactions_to_peers<F: FnMut() -> bool>(
		sync: &mut ChainSync,
		io: &mut dyn SyncIo,
		peers: Vec<PeerId>,
		transactions: Vec<&SignedTransaction>,
		mut should_continue: F,
	) -> HashSet<PeerId> {
		let all_transactions_hashes = transactions.iter()
			.map(|tx| tx.hash())
			.collect::<H256FastSet>();
		let all_transactions_rlp = {
			let mut packet = RlpStream::new_list(transactions.len());
			for tx in &transactions { packet.append(&**tx); }
			packet.out()
		};

		// Clear old transactions from stats
		sync.transactions_stats.retain(&all_transactions_hashes);

		let send_packet = |io: &mut dyn SyncIo, peer_id: PeerId, sent: usize, rlp: Bytes| {
			let size = rlp.len();
			SyncPropagator::send_packet(io, peer_id, TransactionsPacket, rlp);
			trace!(target: "sync", "{:02} <- Transactions ({} entries; {} bytes)", peer_id, sent, size);
		};

		let block_number = io.chain().chain_info().best_block_number;
		let mut sent_to_peers = HashSet::new();
		let mut max_sent = 0;

		// for every peer construct and send transactions packet
		for peer_id in peers {
			if !should_continue() {
				debug!(target: "sync", "Sent up to {} transactions to {} peers.", max_sent, sent_to_peers.len());
				return sent_to_peers;
			}

			let stats = &mut sync.transactions_stats;
			let peer_info = sync.peers.get_mut(&peer_id)
				.expect("peer_id is form peers; peers is result of select_peers_for_transactions; select_peers_for_transactions selects peers from self.peers; qed");

			// Send all transactions, if the peer doesn't know about anything
			if peer_info.last_sent_transactions.is_empty() {
				// update stats
				for hash in &all_transactions_hashes {
					let id = io.peer_session_info(peer_id).and_then(|info| info.id);
					stats.propagated(hash, id, block_number);
				}
				peer_info.last_sent_transactions = all_transactions_hashes.clone();

				send_packet(io, peer_id, all_transactions_hashes.len(), all_transactions_rlp.clone());
				sent_to_peers.insert(peer_id);
				max_sent = cmp::max(max_sent, all_transactions_hashes.len());
				continue;
			}

			// Get hashes of all transactions to send to this peer
			let to_send = all_transactions_hashes.difference(&peer_info.last_sent_transactions)
				.cloned()
				.collect::<HashSet<_>>();
			if to_send.is_empty() {
				continue;
			}

			// Construct RLP
			let (packet, to_send) = {
				let mut to_send = to_send;
				let mut packet = RlpStream::new();
				packet.begin_unbounded_list();
				let mut pushed = 0;
				for tx in &transactions {
					let hash = tx.hash();
					if to_send.contains(&hash) {
						let mut transaction = RlpStream::new();
						tx.rlp_append(&mut transaction);
						let appended = packet.append_raw_checked(&transaction.drain(), 1, MAX_TRANSACTION_PACKET_SIZE);
						if !appended {
							// Maximal packet size reached just proceed with sending
							debug!(target: "sync", "Transaction packet size limit reached. Sending incomplete set of {}/{} transactions.", pushed, to_send.len());
							to_send = to_send.into_iter().take(pushed).collect();
							break;
						}
						pushed += 1;
					}
				}
				packet.finalize_unbounded_list();
				(packet, to_send)
			};

			// Update stats
			let id = io.peer_session_info(peer_id).and_then(|info| info.id);
			for hash in &to_send {
				// update stats
				stats.propagated(hash, id, block_number);
			}

			peer_info.last_sent_transactions = all_transactions_hashes
				.intersection(&peer_info.last_sent_transactions)
				.chain(&to_send)
				.cloned()
				.collect();
			send_packet(io, peer_id, to_send.len(), packet.out());
			sent_to_peers.insert(peer_id);
			max_sent = cmp::max(max_sent, to_send.len());

		}

		debug!(target: "sync", "Sent up to {} transactions to {} peers.", max_sent, sent_to_peers.len());
		sent_to_peers
	}
