	fn it_can_be_started() {
		let temp_path = RandomTempPath::new();
		let mut path = temp_path.as_path().to_owned();
		path.push("pruning");
		path.push("db");

		let spec = get_test_spec();
		let service = ClientService::start(
			ClientConfig::default(),
			&spec,
			&path,
			Arc::new(Miner::with_spec(&spec)),
		);
		assert!(service.is_ok());
		drop(service.unwrap());
		::std::thread::park_timeout(::std::time::Duration::from_millis(100));
	}
