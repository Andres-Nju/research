	fn can_create2() {
		use std::str::FromStr;

		let mut setup = TestSetup::new();
		let state = &mut setup.state;
		let mut tracer = NoopTracer;
		let mut vm_tracer = NoopVMTracer;

		let address = {
			let mut ext = Externalities::new(state, &setup.env_info, &setup.machine, 0, get_test_origin(), &mut setup.sub_state, OutputPolicy::InitContract(None), &mut tracer, &mut vm_tracer, false);
			match ext.create(&U256::max_value(), &U256::zero(), &[], CreateContractAddress::FromSenderSaltAndCodeHash(H256::default())) {
				ContractCreateResult::Created(address, _) => address,
				_ => panic!("Test create failed; expected Created, got Failed/Reverted."),
			}
		};

		assert_eq!(address, Address::from_str("b7c227636666831278bacdb8d7f52933b8698ab9").unwrap());
	}
