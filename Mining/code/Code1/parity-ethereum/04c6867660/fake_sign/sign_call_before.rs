pub fn sign_call(request: CallRequest) -> Result<SignedTransaction, Error> {
	let max_gas = U256::from(50_000_000);
	let gas = match request.gas {
		Some(gas) => gas,
		None => max_gas * 10_u32,
	};
	let from = request.from.unwrap_or_default();

	Ok(Transaction {
		nonce: request.nonce.unwrap_or_default(),
		action: request.to.map_or(Action::Create, Action::Call),
		gas,
		gas_price: request.gas_price.unwrap_or_default(),
		value: request.value.unwrap_or_default(),
		data: request.data.unwrap_or_default(),
	}.fake_sign(from))
}
