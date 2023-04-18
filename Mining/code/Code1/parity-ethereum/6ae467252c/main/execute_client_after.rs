fn execute_client(conf: Configuration, spec: Spec, client_config: ClientConfig) {
	// Setup panic handler
	let panic_handler = PanicHandler::new_in_arc();

	// Setup logging
	let logger = setup_log::setup_log(&conf.args.flag_logging, conf.have_color());
	// Raise fdlimit
	unsafe { ::fdlimit::raise_fd_limit(); }

	info!("Starting {}", paint(Colour::White.bold(), format!("{}", version())));

	let net_settings = conf.net_settings(&spec);
	let sync_config = conf.sync_config(&spec);

	// Create and display a new token for UIs.
	if conf.signer_enabled() && !conf.args.flag_no_token {
		new_token(conf.directories().signer).unwrap_or_else(|e| {
			die!("Error generating token: {:?}", e)
		});
	}

	// Display warning about using unlock with signer
	if conf.signer_enabled() && conf.args.flag_unlock.is_some() {
		warn!("Using Trusted Signer and --unlock is not recommended!");
		warn!("NOTE that Signer will not ask you to confirm transactions from unlocked account.");
	}

	// Secret Store
	let account_service = Arc::new(conf.account_service());

	// Miner
	let miner = Miner::new(conf.miner_options(), conf.spec(), Some(account_service.clone()));
	miner.set_author(conf.author().unwrap_or_default());
	miner.set_gas_floor_target(conf.gas_floor_target());
	miner.set_gas_ceil_target(conf.gas_ceil_target());
	miner.set_extra_data(conf.extra_data());
	miner.set_minimal_gas_price(conf.gas_price());
	miner.set_transactions_limit(conf.args.flag_tx_queue_size);

	// Build client
	let mut service = ClientService::start(
		client_config, spec, net_settings, Path::new(&conf.path()), miner.clone(), !conf.args.flag_no_network
	).unwrap_or_else(|e| die_with_error("Client", e));

	panic_handler.forward_from(&service);
	let client = service.client();

	let external_miner = Arc::new(ExternalMiner::default());
	let network_settings = Arc::new(conf.network_settings());

	// Sync
	let sync = EthSync::new(sync_config, client.clone());
	EthSync::register(&*service.network(), sync.clone()).unwrap_or_else(|e| die_with_error("Error registering eth protocol handler", UtilError::from(e).into()));

	let deps_for_rpc_apis = Arc::new(rpc_apis::Dependencies {
		signer_port: conf.signer_port(),
		signer_queue: Arc::new(rpc_apis::ConfirmationsQueue::default()),
		client: client.clone(),
		sync: sync.clone(),
		secret_store: account_service.clone(),
		miner: miner.clone(),
		external_miner: external_miner.clone(),
		logger: logger.clone(),
		settings: network_settings.clone(),
		allow_pending_receipt_query: !conf.args.flag_geth,
		net_service: service.network(),
	});

	let dependencies = rpc::Dependencies {
		panic_handler: panic_handler.clone(),
		apis: deps_for_rpc_apis.clone(),
	};

	// Setup http rpc
	let rpc_server = rpc::new_http(rpc::HttpConfiguration {
		enabled: network_settings.rpc_enabled,
		interface: conf.rpc_interface(),
		port: network_settings.rpc_port,
		apis: conf.rpc_apis(),
		cors: conf.rpc_cors(),
	}, &dependencies);

	// setup ipc rpc
	let _ipc_server = rpc::new_ipc(conf.ipc_settings(), &dependencies);
	debug!("IPC: {}", conf.ipc_settings());

	if conf.args.flag_webapp { println!("WARNING: Flag -w/--webapp is deprecated. Dapps server is now on by default. Ignoring."); }
	let dapps_server = dapps::new(dapps::Configuration {
		enabled: conf.dapps_enabled(),
		interface: conf.dapps_interface(),
		port: conf.args.flag_dapps_port,
		user: conf.args.flag_dapps_user.clone(),
		pass: conf.args.flag_dapps_pass.clone(),
		dapps_path: conf.directories().dapps,
	}, dapps::Dependencies {
		panic_handler: panic_handler.clone(),
		apis: deps_for_rpc_apis.clone(),
	});

	// Set up a signer
	let signer_server = signer::start(signer::Configuration {
		enabled: conf.signer_enabled(),
		port: conf.args.flag_signer_port,
		signer_path: conf.directories().signer,
	}, signer::Dependencies {
		panic_handler: panic_handler.clone(),
		apis: deps_for_rpc_apis.clone(),
	});

	// Register IO handler
	let io_handler  = Arc::new(ClientIoHandler {
		client: service.client(),
		info: Informant::new(conf.have_color()),
		sync: sync.clone(),
		accounts: account_service.clone(),
		network: Arc::downgrade(&service.network()),
	});
	service.register_io_handler(io_handler).expect("Error registering IO handler");

	if conf.args.cmd_ui {
		if !conf.dapps_enabled() {
			die_with_message("Cannot use UI command with Dapps turned off.");
		}
		url::open(&format!("http://{}:{}/", conf.dapps_interface(), conf.args.flag_dapps_port));
	}

	// Handle exit
	wait_for_exit(panic_handler, rpc_server, dapps_server, signer_server);
}
