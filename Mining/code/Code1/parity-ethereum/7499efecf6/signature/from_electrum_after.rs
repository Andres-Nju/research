	pub fn from_electrum(data: &[u8]) -> Self {
		if data.len() != 65 || data[64] < 27 {
			// fallback to empty (invalid) signature
			return Signature::default();
		}

		let mut sig = [0u8; 65];
		sig.copy_from_slice(data);
		sig[64] -= 27;
		Signature(sig)
	}
