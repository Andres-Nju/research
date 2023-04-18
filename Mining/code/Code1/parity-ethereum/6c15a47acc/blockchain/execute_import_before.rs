fn execute_import(cmd: ImportBlockchain) -> Result<String, String> {
	// Setup panic handler
	let panic_handler = PanicHandler::new_in_arc();

	// load spec file
	let spec = try!(cmd.spec.spec());

	// load genesis hash
	let genesis_hash = spec.genesis_header().hash();

	// Setup logging
	let _logger = setup_log(&cmd.logger_config);

	fdlimit::raise_fd_limit();

	// select pruning algorithm
	let algorithm = cmd.pruning.to_algorithm(&cmd.dirs, genesis_hash, spec.fork_name.as_ref());

	// prepare client_path
	let client_path = cmd.dirs.client_path(genesis_hash, spec.fork_name.as_ref(), algorithm);

	// execute upgrades
	try!(execute_upgrades(&cmd.dirs, genesis_hash, spec.fork_name.as_ref(), algorithm));

	// prepare client config
	let client_config = to_client_config(&cmd.cache_config, &cmd.dirs, genesis_hash, cmd.mode, cmd.tracing, cmd.pruning, cmd.compaction, cmd.vm_type, "".into(), spec.fork_name.as_ref());

	// build client
	let service = try!(ClientService::start(
		client_config,
		spec,
		Path::new(&client_path),
		Arc::new(Miner::with_spec(try!(cmd.spec.spec()))),
	).map_err(|e| format!("Client service error: {:?}", e)));

	panic_handler.forward_from(&service);
	let client = service.client();

	let mut instream: Box<io::Read> = match cmd.file_path {
		Some(f) => Box::new(try!(fs::File::open(&f).map_err(|_| format!("Cannot open given file: {}", f)))),
		None => Box::new(io::stdin()),
	};

	const READAHEAD_BYTES: usize = 8;

	let mut first_bytes: Vec<u8> = vec![0; READAHEAD_BYTES];
	let mut first_read = 0;

	let format = match cmd.format {
		Some(format) => format,
		None => {
			first_read = try!(instream.read(&mut first_bytes).map_err(|_| "Error reading from the file/stream."));
			match first_bytes[0] {
				0xf9 => DataFormat::Binary,
				_ => DataFormat::Hex,
			}
		}
	};

	let informant = Informant::new(client.clone(), None, None, cmd.logger_config.color);

	let do_import = |bytes| {
		while client.queue_info().is_full() { sleep(Duration::from_secs(1)); }
		match client.import_block(bytes) {
			Err(BlockImportError::Import(ImportError::AlreadyInChain)) => {
				trace!("Skipping block already in chain.");
			}
			Err(e) => {
				return Err(format!("Cannot import block: {:?}", e));
			},
			Ok(_) => {},
		}
		informant.tick();
		Ok(())
	};


	match format {
		DataFormat::Binary => {
			loop {
				let mut bytes = if first_read > 0 {first_bytes.clone()} else {vec![0; READAHEAD_BYTES]};
				let n = if first_read > 0 {
					first_read
				} else {
					try!(instream.read(&mut bytes).map_err(|_| "Error reading from the file/stream."))
				};
				if n == 0 { break; }
				first_read = 0;
				let s = try!(PayloadInfo::from(&bytes).map_err(|e| format!("Invalid RLP in the file/stream: {:?}", e))).total();
				bytes.resize(s, 0);
				try!(instream.read_exact(&mut bytes[READAHEAD_BYTES..]).map_err(|_| "Error reading from the file/stream."));
				try!(do_import(bytes));
			}
		}
		DataFormat::Hex => {
			for line in BufReader::new(instream).lines() {
				let s = try!(line.map_err(|_| "Error reading from the file/stream."));
				let s = if first_read > 0 {from_utf8(&first_bytes).unwrap().to_owned() + &(s[..])} else {s};
				first_read = 0;
				let bytes = try!(s.from_hex().map_err(|_| "Invalid hex in file/stream."));
				try!(do_import(bytes));
			}
		}
	}
	client.flush_queue();

	Ok("Import completed.".into())
}
