	fn dispatch_pending(&self, ctx: &BasicContext) {
		if self.pending.read().is_empty() { return }
		let mut pending = self.pending.write();

		debug!(target: "on_demand", "Attempting to dispatch {} pending requests", pending.len());

		// iterate over all pending requests, and check them for hang-up.
		// then, try and find a peer who can serve it.
		let peers = self.peers.read();
		*pending = ::std::mem::replace(&mut *pending, Vec::new()).into_iter()
			.filter(|pending| !pending.sender.is_canceled())
			.filter_map(|pending| {
				// the peer we dispatch to is chosen randomly
				let num_peers = peers.len();
				let rng = rand::random::<usize>() % num_peers;
				for (peer_id, peer) in peers.iter().chain(peers.iter()).skip(rng).take(num_peers) {
					// TODO: see which requests can be answered by the cache?

					if !peer.can_fulfill(&pending.required_capabilities) {
						continue
					}

					match ctx.request_from(*peer_id, pending.net_requests.clone()) {
						Ok(req_id) => {
							trace!(target: "on_demand", "Dispatched request {} to peer {}", req_id, peer_id);
							self.in_transit.write().insert(req_id, pending);
							return None
						}
						Err(net::Error::NoCredits) | Err(net::Error::NotServer) => {}
						Err(e) => debug!(target: "on_demand", "Error dispatching request to peer: {}", e),
					}
				}

				// TODO: maximum number of failures _when we have peers_.
				Some(pending)
			})
			.collect(); // `pending` now contains all requests we couldn't dispatch.

		debug!(target: "on_demand", "Was unable to dispatch {} requests.", pending.len());
	}
