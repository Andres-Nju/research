	fn can_create() {
		use std::str::FromStr;

		let mut setup = TestSetup::new();
		let state = &mut setup.state;
		let mut tracer = NoopTracer;
		let mut vm_tracer = NoopVMTracer;

		let address = {
			let mut ext = Externalities::new(state, &setup.env_info, &setup.machine, &setup.schedule, 0, get_test_origin(), &mut setup.sub_state, OutputPolicy::InitContract(None), &mut tracer, &mut vm_tracer, false);
			match ext.create(&U256::max_value(), &U256::zero(), &[], CreateContractAddress::FromSenderAndNonce) {
				ContractCreateResult::Created(address, _) => address,
				_ => panic!("Test create failed; expected Created, got Failed/Reverted."),
			}
		};

		assert_eq!(address, Address::from_str("bd770416a3345f91e4b34576cb804a576fa48eb1").unwrap());
	}
