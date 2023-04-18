fn execute_light_impl(cmd: RunCmd, logger: Arc<RotatingLogger>) -> Result<RunningClient, String> {
	use light::client as light_client;
	use sync::{LightSyncParams, LightSync, ManageNetwork};
	use parking_lot::{Mutex, RwLock};

	// load spec
	let spec = cmd.spec.spec(SpecParams::new(cmd.dirs.cache.as_ref(), OptimizeFor::Memory))?;

	// load genesis hash
	let genesis_hash = spec.genesis_header().hash();

	// database paths
	let db_dirs = cmd.dirs.database(genesis_hash, cmd.spec.legacy_fork_name(), spec.data_dir.clone());

	// user defaults path
	let user_defaults_path = db_dirs.user_defaults_path();

	// load user defaults
	let user_defaults = UserDefaults::load(&user_defaults_path)?;

	// select pruning algorithm
	let algorithm = cmd.pruning.to_algorithm(&user_defaults);

	// execute upgrades
	execute_upgrades(&cmd.dirs.base, &db_dirs, algorithm, &cmd.compaction)?;

	// create dirs used by parity
	cmd.dirs.create_dirs(cmd.acc_conf.unlocked_accounts.len() == 0, cmd.secretstore_conf.enabled)?;

	//print out running parity environment
	print_running_environment(&spec.data_dir, &cmd.dirs, &db_dirs);

	info!("Running in experimental {} mode.", Colour::Blue.bold().paint("Light Client"));

	// TODO: configurable cache size.
	let cache = LightDataCache::new(Default::default(), Duration::from_secs(60 * GAS_CORPUS_EXPIRATION_MINUTES));
	let cache = Arc::new(Mutex::new(cache));

	// start client and create transaction queue.
	let mut config = light_client::Config {
		queue: Default::default(),
		chain_column: ::ethcore_db::COL_LIGHT_CHAIN,
		verify_full: true,
		check_seal: cmd.check_seal,
		no_hardcoded_sync: cmd.no_hardcoded_sync,
	};

	config.queue.max_mem_use = cmd.cache_config.queue() as usize * 1024 * 1024;
	config.queue.verifier_settings = cmd.verifier_settings;

	// start on_demand service.

	let response_time_window = cmd.on_demand_response_time_window.map_or(
		::light::on_demand::DEFAULT_RESPONSE_TIME_TO_LIVE,
		|s| Duration::from_secs(s)
	);

	let request_backoff_start = cmd.on_demand_request_backoff_start.map_or(
		::light::on_demand::DEFAULT_REQUEST_MIN_BACKOFF_DURATION,
		|s| Duration::from_secs(s)
	);

	let request_backoff_max = cmd.on_demand_request_backoff_max.map_or(
		::light::on_demand::DEFAULT_REQUEST_MAX_BACKOFF_DURATION,
		|s| Duration::from_secs(s)
	);

	let on_demand = Arc::new({
		::light::on_demand::OnDemand::new(
			cache.clone(),
			response_time_window,
			request_backoff_start,
			request_backoff_max,
			cmd.on_demand_request_backoff_rounds_max.unwrap_or(::light::on_demand::DEFAULT_MAX_REQUEST_BACKOFF_ROUNDS),
			cmd.on_demand_request_consecutive_failures.unwrap_or(::light::on_demand::DEFAULT_NUM_CONSECUTIVE_FAILED_REQUESTS)
		)
	});

	let sync_handle = Arc::new(RwLock::new(Weak::new()));
	let fetch = ::light_helpers::EpochFetch {
		on_demand: on_demand.clone(),
		sync: sync_handle.clone(),
	};

	// initialize database.
	let db = db::open_db(&db_dirs.client_path(algorithm).to_str().expect("DB path could not be converted to string."),
						 &cmd.cache_config,
						 &cmd.compaction).map_err(|e| format!("Failed to open database {:?}", e))?;

	let service = light_client::Service::start(config, &spec, fetch, db, cache.clone())
		.map_err(|e| format!("Error starting light client: {}", e))?;
	let client = service.client().clone();
	let txq = Arc::new(RwLock::new(::light::transaction_queue::TransactionQueue::default()));
	let provider = ::light::provider::LightProvider::new(client.clone(), txq.clone());

	// start network.
	// set up bootnodes
	let mut net_conf = cmd.net_conf;
	if !cmd.custom_bootnodes {
		net_conf.boot_nodes = spec.nodes.clone();
	}

	let mut attached_protos = Vec::new();
	let whisper_factory = if cmd.whisper.enabled {
		let whisper_factory = ::whisper::setup(cmd.whisper.target_message_pool_size, &mut attached_protos)
			.map_err(|e| format!("Failed to initialize whisper: {}", e))?;
		whisper_factory
	} else {
		None
	};

	// set network path.
	net_conf.net_config_path = Some(db_dirs.network_path().to_string_lossy().into_owned());
	let sync_params = LightSyncParams {
		network_config: net_conf.into_basic().map_err(|e| format!("Failed to produce network config: {}", e))?,
		client: Arc::new(provider),
		network_id: cmd.network_id.unwrap_or(spec.network_id()),
		subprotocol_name: sync::LIGHT_PROTOCOL,
		handlers: vec![on_demand.clone()],
		attached_protos: attached_protos,
	};
	let light_sync = LightSync::new(sync_params).map_err(|e| format!("Error starting network: {}", e))?;
	let light_sync = Arc::new(light_sync);
	*sync_handle.write() = Arc::downgrade(&light_sync);

	// spin up event loop
	let runtime = Runtime::with_default_thread_count();

	// queue cull service.
	let queue_cull = Arc::new(::light_helpers::QueueCull {
		client: client.clone(),
		sync: light_sync.clone(),
		on_demand: on_demand.clone(),
		txq: txq.clone(),
		executor: runtime.executor(),
	});

	service.register_handler(queue_cull).map_err(|e| format!("Error attaching service: {:?}", e))?;

	// start the network.
	light_sync.start_network();

	// fetch service
	let fetch = fetch::Client::new(FETCH_LIGHT_NUM_DNS_THREADS).map_err(|e| format!("Error starting fetch client: {:?}", e))?;
	let passwords = passwords_from_files(&cmd.acc_conf.password_files)?;

	// prepare account provider
	let account_provider = Arc::new(prepare_account_provider(&cmd.spec, &cmd.dirs, &spec.data_dir, cmd.acc_conf, &passwords)?);
	let rpc_stats = Arc::new(informant::RpcStats::default());

	// the dapps server
	let signer_service = Arc::new(signer::new_service(&cmd.ws_conf, &cmd.logger_config));

	// start RPCs
	let deps_for_rpc_apis = Arc::new(rpc_apis::LightDependencies {
		signer_service: signer_service,
		client: client.clone(),
		sync: light_sync.clone(),
		net: light_sync.clone(),
		secret_store: account_provider,
		logger: logger,
		settings: Arc::new(cmd.net_settings),
		on_demand: on_demand,
		cache: cache.clone(),
		transaction_queue: txq,
		ws_address: cmd.ws_conf.address(),
		fetch: fetch,
		geth_compatibility: cmd.geth_compatibility,
		experimental_rpcs: cmd.experimental_rpcs,
		executor: runtime.executor(),
		whisper_rpc: whisper_factory,
		private_tx_service: None, //TODO: add this to client.
		gas_price_percentile: cmd.gas_price_percentile,
		poll_lifetime: cmd.poll_lifetime
	});

	let dependencies = rpc::Dependencies {
		apis: deps_for_rpc_apis.clone(),
		executor: runtime.executor(),
		stats: rpc_stats.clone(),
	};

	// start rpc servers
	let rpc_direct = rpc::setup_apis(rpc_apis::ApiSet::All, &dependencies);
	let ws_server = rpc::new_ws(cmd.ws_conf, &dependencies)?;
	let http_server = rpc::new_http("HTTP JSON-RPC", "jsonrpc", cmd.http_conf.clone(), &dependencies)?;
	let ipc_server = rpc::new_ipc(cmd.ipc_conf, &dependencies)?;

	// the informant
	let informant = Arc::new(Informant::new(
		LightNodeInformantData {
			client: client.clone(),
			sync: light_sync.clone(),
			cache: cache,
		},
		None,
		Some(rpc_stats),
		cmd.logger_config.color,
	));
	service.add_notify(informant.clone());
	service.register_handler(informant.clone()).map_err(|_| "Unable to register informant handler".to_owned())?;

	Ok(RunningClient {
		inner: RunningClientInner::Light {
			rpc: rpc_direct,
			informant,
			client,
			keep_alive: Box::new((runtime, service, ws_server, http_server, ipc_server)),
		}
	})
}
