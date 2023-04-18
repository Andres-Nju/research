fn guards_delete_folders() {
	let spec = Spec::new_null();
	let path = RandomTempPath::create_dir();
	let mut path = path.as_path().clone();
	let service_params = ServiceParams {
		engine: spec.engine.clone(),
		genesis_block: spec.genesis_block(),
		db_config: DatabaseConfig::with_columns(::db::NUM_COLUMNS),
		pruning: ::util::journaldb::Algorithm::Archive,
		channel: IoChannel::disconnected(),
		snapshot_root: path.clone(),
		db_restore: Arc::new(NoopDBRestore),
	};

	let service = Service::new(service_params).unwrap();
	path.push("restoration");

	let manifest = ManifestData {
		state_hashes: vec![],
		block_hashes: vec![],
		block_number: 0,
		block_hash: Default::default(),
		state_root: Default::default(),
	};

	service.init_restore(manifest.clone(), true).unwrap();
	assert!(path.exists());

	service.abort_restore();
	assert!(!path.exists());

	service.init_restore(manifest.clone(), true).unwrap();
	assert!(path.exists());

	drop(service);
	assert!(!path.exists());
}
