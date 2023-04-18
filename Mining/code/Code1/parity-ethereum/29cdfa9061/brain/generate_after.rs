	fn generate(self) -> Result<KeyPair, Error> {
		let seed = self.0;
		let mut secret = seed.bytes().collect::<Vec<u8>>().keccak256();

		let mut i = 0;
		loop {
			secret = secret.keccak256();
			
			match i > 16384 {
				false => i += 1,
				true => {
					let result = KeyPair::from_secret(secret.clone().into());
					if result.as_ref().ok().map_or(false, |r| r.address()[0] == 0) {
						return result;
					}
				},
			}
		}
	}
