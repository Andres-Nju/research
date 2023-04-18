// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

use std::str::{FromStr, from_utf8};
use std::{io, fs};
use std::io::{BufReader, BufRead};
use std::time::{Instant, Duration};
use std::thread::sleep;
use std::sync::Arc;
use rustc_serialize::hex::FromHex;
use io::{PanicHandler, ForwardPanic};
use util::{ToPretty, Uint, U256, H256, Address, Hashable};
use rlp::PayloadInfo;
use ethcore::service::ClientService;
use ethcore::client::{Mode, DatabaseCompactionProfile, VMType, BlockImportError, BlockChainClient, BlockId};
use ethcore::error::ImportError;
use ethcore::miner::Miner;
use ethcore::verification::queue::VerifierSettings;
use cache::CacheConfig;
use informant::{Informant, MillisecondDuration};
use params::{SpecType, Pruning, Switch, tracing_switch_to_bool, fatdb_switch_to_bool};
use helpers::{to_client_config, execute_upgrades};
use dir::Directories;
use user_defaults::UserDefaults;
use fdlimit;

#[derive(Debug, PartialEq)]
pub enum DataFormat {
	Hex,
	Binary,
}

impl Default for DataFormat {
	fn default() -> Self {
		DataFormat::Binary
	}
}

impl FromStr for DataFormat {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"binary" | "bin" => Ok(DataFormat::Binary),
			"hex" => Ok(DataFormat::Hex),
			x => Err(format!("Invalid format: {}", x))
		}
	}
}

#[derive(Debug, PartialEq)]
pub enum BlockchainCmd {
	Kill(KillBlockchain),
	Import(ImportBlockchain),
	Export(ExportBlockchain),
	ExportState(ExportState),
}

#[derive(Debug, PartialEq)]
pub struct KillBlockchain {
	pub spec: SpecType,
	pub dirs: Directories,
	pub pruning: Pruning,
}

#[derive(Debug, PartialEq)]
pub struct ImportBlockchain {
	pub spec: SpecType,
	pub cache_config: CacheConfig,
	pub dirs: Directories,
	pub file_path: Option<String>,
	pub format: Option<DataFormat>,
	pub pruning: Pruning,
	pub pruning_history: u64,
	pub pruning_memory: usize,
	pub compaction: DatabaseCompactionProfile,
	pub wal: bool,
	pub tracing: Switch,
	pub fat_db: Switch,
	pub vm_type: VMType,
	pub check_seal: bool,
	pub with_color: bool,
	pub verifier_settings: VerifierSettings,
}

#[derive(Debug, PartialEq)]
pub struct ExportBlockchain {
	pub spec: SpecType,
	pub cache_config: CacheConfig,
	pub dirs: Directories,
	pub file_path: Option<String>,
	pub format: Option<DataFormat>,
	pub pruning: Pruning,
	pub pruning_history: u64,
	pub pruning_memory: usize,
	pub compaction: DatabaseCompactionProfile,
	pub wal: bool,
	pub fat_db: Switch,
	pub tracing: Switch,
	pub from_block: BlockId,
	pub to_block: BlockId,
	pub check_seal: bool,
}

#[derive(Debug, PartialEq)]
pub struct ExportState {
	pub spec: SpecType,
	pub cache_config: CacheConfig,
	pub dirs: Directories,
	pub file_path: Option<String>,
	pub format: Option<DataFormat>,
	pub pruning: Pruning,
	pub pruning_history: u64,
	pub pruning_memory: usize,
	pub compaction: DatabaseCompactionProfile,
	pub wal: bool,
	pub fat_db: Switch,
	pub tracing: Switch,
	pub at: BlockId,
	pub storage: bool,
	pub code: bool,
	pub min_balance: Option<U256>,
	pub max_balance: Option<U256>,
}

pub fn execute(cmd: BlockchainCmd) -> Result<(), String> {
	match cmd {
		BlockchainCmd::Kill(kill_cmd) => kill_db(kill_cmd),
		BlockchainCmd::Import(import_cmd) => execute_import(import_cmd),
		BlockchainCmd::Export(export_cmd) => execute_export(export_cmd),
		BlockchainCmd::ExportState(export_cmd) => execute_export_state(export_cmd),
	}
}

fn execute_import(cmd: ImportBlockchain) -> Result<(), String> {
	let timer = Instant::now();

	// Setup panic handler
	let panic_handler = PanicHandler::new_in_arc();

	// load spec file
	let spec = cmd.spec.spec()?;

	// load genesis hash
	let genesis_hash = spec.genesis_header().hash();

	// database paths
	let db_dirs = cmd.dirs.database(genesis_hash, None, spec.data_dir.clone());

	// user defaults path
	let user_defaults_path = db_dirs.user_defaults_path();

	// load user defaults
	let mut user_defaults = UserDefaults::load(&user_defaults_path)?;

	fdlimit::raise_fd_limit();

	// select pruning algorithm
	let algorithm = cmd.pruning.to_algorithm(&user_defaults);

	// check if tracing is on
	let tracing = tracing_switch_to_bool(cmd.tracing, &user_defaults)?;

	// check if fatdb is on
	let fat_db = fatdb_switch_to_bool(cmd.fat_db, &user_defaults, algorithm)?;

	// prepare client and snapshot paths.
	let client_path = db_dirs.client_path(algorithm);
	let snapshot_path = db_dirs.snapshot_path();

	// execute upgrades
	execute_upgrades(&cmd.dirs.base, &db_dirs, algorithm, cmd.compaction.compaction_profile(db_dirs.db_root_path().as_path()))?;

	// create dirs used by parity
	cmd.dirs.create_dirs(false, false, false)?;

	// prepare client config
	let mut client_config = to_client_config(
		&cmd.cache_config,
		spec.name.to_lowercase(),
		Mode::Active,
		tracing,
		fat_db,
		cmd.compaction,
		cmd.wal,
		cmd.vm_type,
		"".into(),
		algorithm,
		cmd.pruning_history,
		cmd.pruning_memory,
		cmd.check_seal
	);

	client_config.queue.verifier_settings = cmd.verifier_settings;

	// build client
	let service = ClientService::start(
		client_config,
		&spec,
		&client_path,
		&snapshot_path,
		&cmd.dirs.ipc_path(),
		Arc::new(Miner::with_spec(&spec)),
	).map_err(|e| format!("Client service error: {:?}", e))?;

	// free up the spec in memory.
	drop(spec);

	panic_handler.forward_from(&service);
	let client = service.client();

	let mut instream: Box<io::Read> = match cmd.file_path {
		Some(f) => Box::new(fs::File::open(&f).map_err(|_| format!("Cannot open given file: {}", f))?),
		None => Box::new(io::stdin()),
	};

	const READAHEAD_BYTES: usize = 8;

	let mut first_bytes: Vec<u8> = vec![0; READAHEAD_BYTES];
	let mut first_read = 0;

	let format = match cmd.format {
		Some(format) => format,
		None => {
			first_read = instream.read(&mut first_bytes).map_err(|_| "Error reading from the file/stream.")?;
			match first_bytes[0] {
				0xf9 => DataFormat::Binary,
				_ => DataFormat::Hex,
			}
		}
	};

	let informant = Arc::new(Informant::new(client.clone(), None, None, None, None, cmd.with_color));
	service.register_io_handler(informant).map_err(|_| "Unable to register informant handler".to_owned())?;

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
		Ok(())
	};

	match format {
		DataFormat::Binary => {
			loop {
				let mut bytes = if first_read > 0 {first_bytes.clone()} else {vec![0; READAHEAD_BYTES]};
				let n = if first_read > 0 {
					first_read
				} else {
					instream.read(&mut bytes).map_err(|_| "Error reading from the file/stream.")?
				};
				if n == 0 { break; }
				first_read = 0;
				let s = PayloadInfo::from(&bytes).map_err(|e| format!("Invalid RLP in the file/stream: {:?}", e))?.total();
				bytes.resize(s, 0);
				instream.read_exact(&mut bytes[n..]).map_err(|_| "Error reading from the file/stream.")?;
				do_import(bytes)?;
			}
		}
		DataFormat::Hex => {
			for line in BufReader::new(instream).lines() {
				let s = line.map_err(|_| "Error reading from the file/stream.")?;
				let s = if first_read > 0 {from_utf8(&first_bytes).unwrap().to_owned() + &(s[..])} else {s};
				first_read = 0;
				let bytes = s.from_hex().map_err(|_| "Invalid hex in file/stream.")?;
				do_import(bytes)?;
			}
		}
	}
	client.flush_queue();

	// save user defaults
	user_defaults.pruning = algorithm;
	user_defaults.tracing = tracing;
	user_defaults.fat_db = fat_db;
	user_defaults.save(&user_defaults_path)?;

	let report = client.report();

	let ms = timer.elapsed().as_milliseconds();
	info!("Import completed in {} seconds, {} blocks, {} blk/s, {} transactions, {} tx/s, {} Mgas, {} Mgas/s",
		ms / 1000,
		report.blocks_imported,
		(report.blocks_imported * 1000) as u64 / ms,
		report.transactions_applied,
		(report.transactions_applied * 1000) as u64 / ms,
		report.gas_processed / From::from(1_000_000),
		(report.gas_processed / From::from(ms * 1000)).low_u64(),
	);
	Ok(())
}

fn start_client(
	dirs: Directories,
	spec: SpecType,
	pruning: Pruning,
	pruning_history: u64,
	pruning_memory: usize,
	tracing: Switch,
	fat_db: Switch,
	compaction: DatabaseCompactionProfile,
	wal: bool,
	cache_config: CacheConfig
) -> Result<ClientService, String> {

	// load spec file
	let spec = spec.spec()?;

	// load genesis hash
	let genesis_hash = spec.genesis_header().hash();

	// database paths
	let db_dirs = dirs.database(genesis_hash, None, spec.data_dir.clone());

	// user defaults path
	let user_defaults_path = db_dirs.user_defaults_path();

	// load user defaults
	let user_defaults = UserDefaults::load(&user_defaults_path)?;

	fdlimit::raise_fd_limit();

	// select pruning algorithm
	let algorithm = pruning.to_algorithm(&user_defaults);

	// check if tracing is on
	let tracing = tracing_switch_to_bool(tracing, &user_defaults)?;

	// check if fatdb is on
	let fat_db = fatdb_switch_to_bool(fat_db, &user_defaults, algorithm)?;

	// prepare client and snapshot paths.
	let client_path = db_dirs.client_path(algorithm);
	let snapshot_path = db_dirs.snapshot_path();

	// execute upgrades
	execute_upgrades(&dirs.base, &db_dirs, algorithm, compaction.compaction_profile(db_dirs.db_root_path().as_path()))?;

	// create dirs used by parity
	dirs.create_dirs(false, false, false)?;

	// prepare client config
	let client_config = to_client_config(
		&cache_config,
		spec.name.to_lowercase(),
		Mode::Active,
		tracing,
		fat_db,
		compaction,
		wal,
		VMType::default(),
		"".into(),
		algorithm,
		pruning_history,
		pruning_memory,
		true,
	);

	let service = ClientService::start(
		client_config,
		&spec,
		&client_path,
		&snapshot_path,
		&dirs.ipc_path(),
		Arc::new(Miner::with_spec(&spec)),
	).map_err(|e| format!("Client service error: {:?}", e))?;

	drop(spec);
	Ok(service)
}

fn execute_export(cmd: ExportBlockchain) -> Result<(), String> {
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
	let format = cmd.format.unwrap_or_default();

	panic_handler.forward_from(&service);
	let client = service.client();

	let mut out: Box<io::Write> = match cmd.file_path {
		Some(f) => Box::new(fs::File::create(&f).map_err(|_| format!("Cannot write to file given: {}", f))?),
		None => Box::new(io::stdout()),
	};

	let from = client.block_number(cmd.from_block).ok_or("From block could not be found")?;
	let to = client.block_number(cmd.to_block).ok_or("To block could not be found")?;

	for i in from..(to + 1) {
		if i % 10000 == 0 {
			info!("#{}", i);
		}
		let b = client.block(BlockId::Number(i)).ok_or("Error exporting incomplete chain")?.into_inner();
		match format {
			DataFormat::Binary => { out.write(&b).expect("Couldn't write to stream."); }
			DataFormat::Hex => { out.write_fmt(format_args!("{}", b.pretty())).expect("Couldn't write to stream."); }
		}
	}

	info!("Export completed.");
	Ok(())
}

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

pub fn kill_db(cmd: KillBlockchain) -> Result<(), String> {
	let spec = cmd.spec.spec()?;
	let genesis_hash = spec.genesis_header().hash();
	let db_dirs = cmd.dirs.database(genesis_hash, None, spec.data_dir);
	let user_defaults_path = db_dirs.user_defaults_path();
	let user_defaults = UserDefaults::load(&user_defaults_path)?;
	let algorithm = cmd.pruning.to_algorithm(&user_defaults);
	let dir = db_dirs.db_path(algorithm);
	fs::remove_dir_all(&dir).map_err(|e| format!("Error removing database: {:?}", e))?;
	info!("Database deleted.");
	Ok(())
}

#[cfg(test)]
mod test {
	use super::DataFormat;

	#[test]
	fn test_data_format_parsing() {
		assert_eq!(DataFormat::Binary, "binary".parse().unwrap());
		assert_eq!(DataFormat::Binary, "bin".parse().unwrap());
		assert_eq!(DataFormat::Hex, "hex".parse().unwrap());
	}
}
