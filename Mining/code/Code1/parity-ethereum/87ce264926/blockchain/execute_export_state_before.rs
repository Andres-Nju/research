fn execute_export_state(cmd: ExportState) -> Result<(), String> {
	// Setup panic handler
	let service = start_client(
		cmd.dirs,
		cmd.spec,
		cmd.pruning,
		cmd.pruning_history,
		cmd.pruning_memory,
		cmd.tracing,
		cmd.fat_db,
		cmd.compaction,
		cmd.wal,
		cmd.cache_config
	)?;

	let panic_handler = PanicHandler::new_in_arc();

	panic_handler.forward_from(&service);
	let client = service.client();

	let mut out: Box<io::Write> = match cmd.file_path {
		Some(f) => Box::new(fs::File::create(&f).map_err(|_| format!("Cannot write to file given: {}", f))?),
		None => Box::new(io::stdout()),
	};

	let mut last: Option<Address> = None;
	let at = cmd.at;
	let mut i = 0usize;

	out.write_fmt(format_args!("{{ \"state\": [", )).expect("Couldn't write to stream.");
	loop {
		let accounts = client.list_accounts(at, last.as_ref(), 1000).ok_or("Specified block not found")?;
		if accounts.is_empty() {
			break;
		}

		for account in accounts.into_iter() {
			let balance = client.balance(&account, at).unwrap_or_else(U256::zero);
			if cmd.min_balance.map_or(false, |m| balance < m) || cmd.max_balance.map_or(false, |m| balance > m) {
				last = Some(account);
				continue; //filtered out
			}

			if i != 0 {
				out.write(b",").expect("Write error");
			}
			out.write_fmt(format_args!("\n\"0x{}\": {{\"balance\": \"{:x}\", \"nonce\": \"{:x}\"", account.hex(), balance, client.nonce(&account, at).unwrap_or_else(U256::zero))).expect("Write error");
			let code = client.code(&account, at).unwrap_or(None).unwrap_or_else(Vec::new);
			if !code.is_empty() {
				out.write_fmt(format_args!(", \"code_hash\": \"0x{}\"", code.sha3().hex())).expect("Write error");
				if cmd.code {
					out.write_fmt(format_args!(", \"code\": \"{}\"", code.to_hex())).expect("Write error");
				}
			}
			let storage_root = client.storage_root(&account, at).unwrap_or(::util::SHA3_NULL_RLP);
			if storage_root != ::util::SHA3_NULL_RLP {
				out.write_fmt(format_args!(", \"storage_root\": \"0x{}\"", storage_root.hex())).expect("Write error");
				if cmd.storage {
					out.write_fmt(format_args!(", \"storage\": {{")).expect("Write error");
					let mut last_storage: Option<H256> = None;
					loop {
						let keys = client.list_storage(at, &account, last_storage.as_ref(), 1000).ok_or("Specified block not found")?;
						if keys.is_empty() {
							break;
						}

						let mut si = 0;
						for key in keys.into_iter() {
							if si != 0 {
								out.write(b",").expect("Write error");
							}
							out.write_fmt(format_args!("\n\t\"0x{}\": \"0x{}\"", key.hex(), client.storage_at(&account, &key, at).unwrap_or_else(Default::default).hex())).expect("Write error");
							si += 1;
							last_storage = Some(key);
						}
					}
					out.write(b"\n}").expect("Write error");
				}
			}
			out.write(b"}").expect("Write error");
			i += 1;
			if i % 10000 == 0 {
				info!("Account #{}", i);
			}
			last = Some(account);
		}
	}
	out.write_fmt(format_args!("\n]}}")).expect("Write error");
	info!("Export completed.");
	Ok(())
}
