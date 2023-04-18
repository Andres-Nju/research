fn execute_export(conf: Configuration) {
	// Setup panic handler
	let panic_handler = PanicHandler::new_in_arc();

	// Setup logging
	let _logger = setup_log::setup_log(&conf.args.flag_logging, conf.args.flag_no_color);
	// Raise fdlimit
	unsafe { ::fdlimit::raise_fd_limit(); }

	let spec = conf.spec();
	let net_settings = NetworkConfiguration {
		config_path: None,
		listen_address: None,
		public_address: None,
		udp_port: None,
		nat_enabled: false,
		discovery_enabled: false,
		boot_nodes: Vec::new(),
		use_secret: None,
		ideal_peers: 0,
		reserved_nodes: Vec::new(),
		non_reserved_mode: ::util::network::NonReservedPeerMode::Accept,
	};
	let client_config = conf.client_config(&spec);

	// Build client
	let service = ClientService::start(
		client_config, spec, net_settings, Path::new(&conf.path()), Arc::new(Miner::with_spec(conf.spec())), false
	).unwrap_or_else(|e| die_with_error("Client", e));

	panic_handler.forward_from(&service);
	let client = service.client();

	// we have a client!
	let parse_block_id = |s: &str, arg: &str| -> u64 {
		if s == "latest" {
			client.chain_info().best_block_number
		} else if let Ok(n) = s.parse::<u64>() {
			n
		} else if let Ok(h) = H256::from_str(s) {
			client.block_number(BlockID::Hash(h)).unwrap_or_else(|| {
				die!("Unknown block hash passed to {} parameter: {:?}", arg, s);
			})
		} else {
			die!("Invalid {} parameter given: {:?}", arg, s);
		}
	};
	let from = parse_block_id(&conf.args.flag_from, "--from");
	let to = parse_block_id(&conf.args.flag_to, "--to");
	let format = match conf.args.flag_format {
		Some(x) => match x.deref() {
			"binary" | "bin" => DataFormat::Binary,
			"hex" => DataFormat::Hex,
			x => die!("Invalid --format parameter given: {:?}", x),
		},
		None if conf.args.arg_file.is_none() => DataFormat::Hex,
		None => DataFormat::Binary,
	};

	let mut out: Box<Write> = if let Some(f) = conf.args.arg_file {
		Box::new(File::create(&f).unwrap_or_else(|_| die!("Cannot write to file given: {}", f)))
	} else {
		Box::new(::std::io::stdout())
	};

	for i in from..(to + 1) {
		let b = client.deref().block(BlockID::Number(i)).unwrap();
		match format {
			DataFormat::Binary => { out.write(&b).expect("Couldn't write to stream."); }
			DataFormat::Hex => { out.write_fmt(format_args!("{}", b.pretty())).expect("Couldn't write to stream."); }
		}
	}
}
