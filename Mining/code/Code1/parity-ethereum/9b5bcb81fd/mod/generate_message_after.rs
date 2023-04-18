	fn generate_message(&self, block_hash: Option<BlockHash>) -> Option<Bytes> {
		let h = self.height.load(AtomicOrdering::SeqCst);
		let r = self.view.load(AtomicOrdering::SeqCst);
		let s = *self.step.read();
		let vote_info = message_info_rlp(&VoteStep::new(h, r, s), block_hash);
		match self.signer.sign(vote_info.sha3()).map(Into::into) {
			Ok(signature) => {
				let message_rlp = message_full_rlp(&signature, &vote_info);
				let message = ConsensusMessage::new(signature, h, r, s, block_hash);
				let validator = self.signer.address();
				self.votes.vote(message.clone(), &validator);
				debug!(target: "engine", "Generated {:?} as {}.", message, validator);
				self.handle_valid_message(&message);

				Some(message_rlp)
			},
			Err(e) => {
				trace!(target: "engine", "Could not sign the message {}", e);
				None
			},
		}
	}
