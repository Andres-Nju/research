	fn registrar_address(&self) -> Option<Address>;

	/// Get address from registrar for the specified key.
	fn get_address(&self, key: &str, block: BlockId) -> Result<Option<Address>, String> {
		use registrar::registrar::functions::get_address::{encode_input, decode_output};

		let registrar_address = match self.registrar_address() {
			Some(address) => address,
			None => return Err("Registrar address not defined.".to_owned())
		};

		let hashed_key: [u8; 32] = keccak(key).into();
		let id = encode_input(hashed_key, DNS_A_RECORD);

		let address_bytes = self.call_contract(block, registrar_address, id)?;

		let address = decode_output(&address_bytes).map_err(|e| e.to_string())?;

		if address.is_zero() {
			Ok(None)
		} else {
			Ok(Some(address))
		}
	}
