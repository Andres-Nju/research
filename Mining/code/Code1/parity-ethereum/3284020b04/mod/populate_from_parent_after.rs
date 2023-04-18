	fn populate_from_parent(&self, header: &mut Header, parent: &Header) {
		let new_difficulty = U256::from(U128::max_value()) + header_step(parent).expect("Header has been verified; qed").into() - self.step.load().into();
		header.set_difficulty(new_difficulty);
	}

