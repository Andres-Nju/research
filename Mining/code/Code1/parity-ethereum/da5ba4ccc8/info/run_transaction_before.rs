pub fn run_transaction<T: Informant>(
	name: &str,
	idx: usize,
	spec: &ethjson::state::test::ForkSpec,
	pre_state: &pod_state::PodState,
	post_root: H256,
	env_info: &client::EnvInfo,
	transaction: transaction::SignedTransaction,
	mut informant: T,
) {
	let spec_name = format!("{:?}", spec).to_lowercase();
	let spec = match EvmTestClient::spec_from_json(spec) {
		Some(spec) => {
			informant.before_test(&format!("{}:{}:{}", name, spec_name, idx), "starting");
			spec
		},
		None => {
			informant.before_test(&format!("{}:{}:{}", name, spec_name, idx), "skipping because of missing spec");
			return;
		},
	};

	informant.set_gas(env_info.gas_limit);

	let result = run(&spec, env_info.gas_limit, pre_state, |mut client| {
		let result = client.transact(env_info, transaction, trace::NoopTracer, informant);
		match result {
			TransactResult::Ok { state_root, gas_left, .. } if state_root != post_root => {
				(Err(EvmTestError::PostCondition(format!(
					"State root mismatch (got: 0x{:x}, expected: 0x{:x})",
					state_root,
					post_root,
				))), Some(gas_left), None)
			},
			TransactResult::Ok { state_root, gas_left, output, vm_trace, .. } => {
				(Ok((state_root, output)), Some(gas_left), vm_trace)
			},
			TransactResult::Err { error, .. } => {
				(Err(EvmTestError::PostCondition(format!(
					"Unexpected execution error: {:?}", error
				))), None, None)
			},
		}
	});

	T::finish(result)
}
