	fn push_work_all(&self, payload: String, tcp_dispatcher: &Dispatcher) {
		let hup_peers = {
			let workers = self.workers.read();
			let next_request_id = {
				let mut counter = self.notify_counter.write();
				if *counter == ::std::u32::MAX {
					*counter = NOTIFY_COUNTER_INITIAL;
				} else {
					*counter = *counter + 1
				}
				*counter
			};

			let mut hup_peers = HashSet::new();
			let workers_msg = format!("{{ \"id\": {}, \"method\": \"mining.notify\", \"params\": {} }}", next_request_id, payload);
			trace!(target: "stratum", "pushing work for {} workers (payload: '{}')", workers.len(), &workers_msg);
			for (addr, _) in workers.iter() {
				trace!(target: "stratum", "pushing work to {}", addr);
				match tcp_dispatcher.push_message(addr, workers_msg.clone()) {
					Err(PushMessageError::NoSuchPeer) => {
						trace!(target: "stratum", "Worker no longer connected: {}", addr);
						hup_peers.insert(addr.clone());
					},
					Err(e) => {
						warn!(target: "stratum", "Unexpected transport error: {:?}", e);
					},
					Ok(_) => {},
				}
			}
			hup_peers
		};

		if !hup_peers.is_empty() {
			let mut workers = self.workers.write();
			for hup_peer in hup_peers {
				workers.remove(&hup_peer);
			}
		}
	}
