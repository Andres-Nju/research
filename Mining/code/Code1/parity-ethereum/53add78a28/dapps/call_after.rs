	fn call(&self, address: Address, data: Bytes) -> BoxFuture<Bytes, String> {
		let (header, env_info) = (self.client.best_block_header(), self.client.latest_env_info());

		let maybe_future = self.sync.with_context(move |ctx| {
			self.on_demand
				.request(ctx, on_demand::request::TransactionProof {
					tx: Transaction {
						nonce: self.client.engine().account_start_nonce(),
						action: Action::Call(address),
						gas: 50_000_000.into(),
						gas_price: 0.into(),
						value: 0.into(),
						data: data,
					}.fake_sign(Address::default()),
					header: on_demand::request::HeaderRef::Stored(header),
					env_info: env_info,
					engine: self.client.engine().clone(),
				})
				.expect("todo: handle error")
				.then(|res| match res {
					Ok(Ok(executed)) => Ok(executed.output),
					Ok(Err(e)) => Err(format!("Failed to execute transaction: {}", e)),
					Err(_) => Err(format!("On-demand service dropped request unexpectedly.")),
				})
		});

		match maybe_future {
			Some(fut) => fut.boxed(),
			None => future::err("cannot query registry: network disabled".into()).boxed(),
		}
	}
