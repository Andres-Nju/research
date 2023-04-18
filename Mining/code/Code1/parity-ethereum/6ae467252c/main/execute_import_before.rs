fn execute_import(conf: Configuration) {
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

	let mut instream: Box<Read> = if let Some(ref f) = conf.args.arg_file {
		let f = File::open(f).unwrap_or_else(|_| die!("Cannot open the file given: {}", f));
		Box::new(f)
	} else {
		Box::new(::std::io::stdin())
	};

	const READAHEAD_BYTES: usize = 8;

	let mut first_bytes: Bytes = vec![0; READAHEAD_BYTES];
	let mut first_read = 0;

	let format = match conf.args.flag_format {
		Some(ref x) => match x.deref() {
			"binary" | "bin" => DataFormat::Binary,
			"hex" => DataFormat::Hex,
			x => die!("Invalid --format parameter given: {:?}", x),
		},
		None => {
			// autodetect...
			first_read = instream.read(&mut(first_bytes[..])).unwrap_or_else(|_| die!("Error reading from the file/stream."));
			match first_bytes[0] {
				0xf9 => {
					println!("Autodetected binary data format.");
					DataFormat::Binary
				}
				_ => {
					println!("Autodetected hex data format.");
					DataFormat::Hex
				}
			}
		}
	};

	let informant = Informant::new(conf.have_color());

	let do_import = |bytes| {
		while client.queue_info().is_full() { sleep(Duration::from_secs(1)); }
		match client.import_block(bytes) {
			Ok(_) => {}
			Err(Error::Import(ImportError::AlreadyInChain)) => { trace!("Skipping block already in chain."); }
			Err(e) => die!("Cannot import block: {:?}", e)
		}
		informant.tick(client.deref(), None);
	};

	match format {
		DataFormat::Binary => {
			loop {
				let mut bytes: Bytes = if first_read > 0 {first_bytes.clone()} else {vec![0; READAHEAD_BYTES]};
				let n = if first_read > 0 {first_read} else {instream.read(&mut(bytes[..])).unwrap_or_else(|_| die!("Error reading from the file/stream."))};
				if n == 0 { break; }
				first_read = 0;
				let s = PayloadInfo::from(&(bytes[..])).unwrap_or_else(|e| die!("Invalid RLP in the file/stream: {:?}", e)).total();
				bytes.resize(s, 0);
				instream.read_exact(&mut(bytes[READAHEAD_BYTES..])).unwrap_or_else(|_| die!("Error reading from the file/stream."));
				do_import(bytes);
			}
		}
		DataFormat::Hex => {
			for line in BufReader::new(instream).lines() {
				let s = line.unwrap_or_else(|_| die!("Error reading from the file/stream."));
				let s = if first_read > 0 {from_utf8(&first_bytes).unwrap().to_owned() + &(s[..])} else {s};
				first_read = 0;
				let bytes = FromHex::from_hex(&(s[..])).unwrap_or_else(|_| die!("Invalid hex in file/stream."));
				do_import(bytes);
			}
		}
	}
	client.flush_queue();
}
