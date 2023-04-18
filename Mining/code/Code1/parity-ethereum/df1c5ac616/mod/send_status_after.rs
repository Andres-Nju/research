	fn send_status(&mut self, io: &mut dyn SyncIo, peer: PeerId) -> Result<(), network::Error> {
		let warp_protocol_version = io.protocol_version(&WARP_SYNC_PROTOCOL_ID, peer);
		let warp_protocol = warp_protocol_version != 0;
		let private_tx_protocol = warp_protocol_version >= PAR_PROTOCOL_VERSION_3.0;
		let protocol = if warp_protocol { warp_protocol_version } else { ETH_PROTOCOL_VERSION_63.0 };
		trace!(target: "sync", "Sending status to {}, protocol version {}", peer, protocol);
		let mut packet = RlpStream::new();
		packet.begin_unbounded_list();
		let chain = io.chain().chain_info();
		packet.append(&(protocol as u32));
		packet.append(&self.network_id);
		packet.append(&chain.total_difficulty);
		packet.append(&chain.best_block_hash);
		packet.append(&chain.genesis_hash);
		if warp_protocol {
			let manifest = io.snapshot_service().manifest();
			let block_number = manifest.as_ref().map_or(0, |m| m.block_number);
			let manifest_hash = manifest.map_or(H256::zero(), |m| keccak(m.into_rlp()));
			packet.append(&manifest_hash);
			packet.append(&block_number);
			if private_tx_protocol {
				packet.append(&self.private_tx_handler.is_some());
			}
		}
		packet.finalize_unbounded_list();
		io.respond(StatusPacket.id(), packet.out())
	}
