fn restored_is_equivalent() {
	const NUM_BLOCKS: u32 = 400;
	const TX_PER: usize = 5;

	let gas_prices = vec![1.into(), 2.into(), 3.into(), 999.into()];

	let client = generate_dummy_client_with_spec_and_data(Spec::new_null, NUM_BLOCKS, TX_PER, &gas_prices);

	let path = RandomTempPath::create_dir();
	let mut path = path.as_path().clone();
	let mut client_db = path.clone();

	client_db.push("client_db");
	path.push("snapshot");

	let db_config = DatabaseConfig::with_columns(::db::NUM_COLUMNS);

	let spec = Spec::new_null();
	let client2 = Client::new(
		Default::default(),
		&spec,
		&client_db,
		Arc::new(::miner::Miner::with_spec(&spec)),
		IoChannel::disconnected(),
		&db_config,
	).unwrap();

	let service_params = ServiceParams {
		engine: spec.engine.clone(),
		genesis_block: spec.genesis_block(),
		db_config: db_config,
		pruning: ::util::journaldb::Algorithm::Archive,
		channel: IoChannel::disconnected(),
		snapshot_root: path,
		db_restore: client2.clone(),
	};

	let service = Service::new(service_params).unwrap();
	service.take_snapshot(&client, NUM_BLOCKS as u64).unwrap();

	let manifest = service.manifest().unwrap();

	service.init_restore(manifest.clone(), true).unwrap();
	assert!(service.init_restore(manifest.clone(), true).is_ok());

	for hash in manifest.state_hashes {
		let chunk = service.chunk(hash).unwrap();
		service.feed_state_chunk(hash, &chunk);
	}

	for hash in manifest.block_hashes {
		let chunk = service.chunk(hash).unwrap();
		service.feed_block_chunk(hash, &chunk);
	}

	assert_eq!(service.status(), ::snapshot::RestorationStatus::Inactive);

	for x in 0..NUM_BLOCKS {
		let block1 = client.block(BlockID::Number(x as u64)).unwrap();
		let block2 = client2.block(BlockID::Number(x as u64)).unwrap();

		assert_eq!(block1, block2);
	}
}
