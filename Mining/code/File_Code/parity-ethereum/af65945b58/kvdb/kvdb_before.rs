// Copyright 2015, 2016 Ethcore (UK) Ltd.
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

//! Key-Value store abstraction with `RocksDB` backend.

use std::default::Default;
use rocksdb::{DB, Writable, WriteBatch, IteratorMode, DBVector, DBIterator,
	IndexType, Options, DBCompactionStyle, BlockBasedOptions, Direction};

const DB_BACKGROUND_FLUSHES: i32 = 2;
const DB_BACKGROUND_COMPACTIONS: i32 = 2;

/// Write transaction. Batches a sequence of put/delete operations for efficiency.
pub struct DBTransaction {
	batch: WriteBatch,
}

impl Default for DBTransaction {
	fn default() -> Self {
		DBTransaction::new()
	}
}

impl DBTransaction {
	/// Create new transaction.
	pub fn new() -> DBTransaction {
		DBTransaction { batch: WriteBatch::new() }
	}

	/// Insert a key-value pair in the transaction. Any existing value value will be overwritten upon write.
	pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
		self.batch.put(key, value)
	}

	/// Delete value by key.
	pub fn delete(&self, key: &[u8]) -> Result<(), String> {
		self.batch.delete(key)
	}
}

/// Compaction profile for the database settings
pub struct CompactionProfile {
	/// L0-L1 target file size
	pub initial_file_size: u64,
	/// L2-LN target file size multiplier
	pub file_size_multiplier: i32,
	/// rate limiter for background flushes and compactions, bytes/sec, if any
	pub write_rate_limit: Option<u64>,
}

impl CompactionProfile {
	/// Default profile suitable for most storage
	pub fn default() -> CompactionProfile {
		CompactionProfile {
			initial_file_size: 32 * 1024 * 1024,
			file_size_multiplier: 2,
			write_rate_limit: None,
		}
	}

	/// Slow hdd compaction profile
	pub fn hdd() -> CompactionProfile {
		CompactionProfile {
			initial_file_size: 192 * 1024 * 1024,
			file_size_multiplier: 1,
			write_rate_limit: Some(8 * 1024 * 1024),
		}
	}
}

/// Database configuration
pub struct DatabaseConfig {
	/// Optional prefix size in bytes. Allows lookup by partial key.
	pub prefix_size: Option<usize>,
	/// Max number of open files.
	pub max_open_files: i32,
	/// Cache-size
	pub cache_size: Option<usize>,
	/// Compaction profile
	pub compaction: CompactionProfile,
}

impl DatabaseConfig {
	/// Database with default settings and specified cache size
	pub fn with_cache(cache_size: usize) -> DatabaseConfig {
		DatabaseConfig {
			cache_size: Some(cache_size),
			prefix_size: None,
			max_open_files: -1,
			compaction: CompactionProfile::default(),
		}
	}

	/// Modify the compaction profile
	pub fn compaction(mut self, profile: CompactionProfile) -> Self {
		self.compaction = profile;
		self
	}

	/// Modify the prefix of the db
	pub fn prefix(mut self, prefix_size: usize) -> Self {
		self.prefix_size = Some(prefix_size);
		self
	}
}

impl Default for DatabaseConfig {
	fn default() -> DatabaseConfig {
		DatabaseConfig {
			cache_size: None,
			prefix_size: None,
			max_open_files: -1,
			compaction: CompactionProfile::default(),
		}
	}
}

/// Database iterator
pub struct DatabaseIterator {
	iter: DBIterator,
}

impl<'a> Iterator for DatabaseIterator {
	type Item = (Box<[u8]>, Box<[u8]>);

    fn next(&mut self) -> Option<Self::Item> {
		self.iter.next()
	}
}

/// Key-Value database.
pub struct Database {
	db: DB,
}

impl Database {
	/// Open database with default settings.
	pub fn open_default(path: &str) -> Result<Database, String> {
		Database::open(&DatabaseConfig::default(), path)
	}

	/// Open database file. Creates if it does not exist.
	pub fn open(config: &DatabaseConfig, path: &str) -> Result<Database, String> {
		let mut opts = Options::new();
		if let Some(rate_limit) = config.compaction.write_rate_limit {
			try!(opts.set_parsed_options(&format!("rate_limiter_bytes_per_sec={}", rate_limit)));
		}
		opts.set_max_open_files(config.max_open_files);
		opts.create_if_missing(true);
		opts.set_use_fsync(false);

		// compaction settings
		opts.set_compaction_style(DBCompactionStyle::DBUniversalCompaction);
		opts.set_target_file_size_base(config.compaction.initial_file_size);
		opts.set_target_file_size_multiplier(config.compaction.file_size_multiplier);

		opts.set_max_background_flushes(DB_BACKGROUND_FLUSHES);
		opts.set_max_background_compactions(DB_BACKGROUND_COMPACTIONS);
		if let Some(cache_size) = config.cache_size {
			// half goes to read cache
			opts.set_block_cache_size_mb(cache_size as u64 / 2);
			// quarter goes to each of the two write buffers
			opts.set_write_buffer_size(cache_size * 1024 * 256);
		}
		/*
		opts.set_bytes_per_sync(8388608);
		opts.set_disable_data_sync(false);
		opts.set_block_cache_size_mb(1024);
		opts.set_table_cache_num_shard_bits(6);
		opts.set_max_write_buffer_number(32);
		opts.set_write_buffer_size(536870912);
		opts.set_target_file_size_base(1073741824);
		opts.set_min_write_buffer_number_to_merge(4);
		opts.set_level_zero_stop_writes_trigger(2000);
		opts.set_level_zero_slowdown_writes_trigger(0);
		opts.set_compaction_style(DBUniversalCompaction);
		opts.set_max_background_compactions(4);
		opts.set_max_background_flushes(4);
		opts.set_filter_deletes(false);
		opts.set_disable_auto_compactions(false);
		*/

		if let Some(size) = config.prefix_size {
			let mut block_opts = BlockBasedOptions::new();
			block_opts.set_index_type(IndexType::HashSearch);
			opts.set_block_based_table_factory(&block_opts);
			opts.set_prefix_extractor_fixed_size(size);
		}
		let db = match DB::open(&opts, path) {
			Ok(db) => db,
			Err(ref s) if s.starts_with("Corruption:") => {
				info!("{}", s);
				info!("Attempting DB repair for {}", path);
				try!(DB::repair(&opts, path));
				try!(DB::open(&opts, path))
			},
			Err(s) => { return Err(s); }
		};
		Ok(Database { db: db })
	}

	/// Insert a key-value pair in the transaction. Any existing value value will be overwritten.
	pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
		self.db.put(key, value)
	}

	/// Delete value by key.
	pub fn delete(&self, key: &[u8]) -> Result<(), String> {
		self.db.delete(key)
	}

	/// Commit transaction to database.
	pub fn write(&self, tr: DBTransaction) -> Result<(), String> {
		self.db.write(tr.batch)
	}

	/// Get value by key.
	pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, String> {
		self.db.get(key)
	}

	/// Get value by partial key. Prefix size should match configured prefix size.
	pub fn get_by_prefix(&self, prefix: &[u8]) -> Option<Box<[u8]>> {
		let mut iter = self.db.iterator(IteratorMode::From(prefix, Direction::Forward));
		match iter.next() {
			// TODO: use prefix_same_as_start read option (not availabele in C API currently)
			Some((k, v)) => if k[0 .. prefix.len()] == prefix[..] { Some(v) } else { None },
			_ => None
		}
	}

	/// Check if there is anything in the database.
	pub fn is_empty(&self) -> bool {
		self.db.iterator(IteratorMode::Start).next().is_none()
	}

	/// Check if there is anything in the database.
	pub fn iter(&self) -> DatabaseIterator {
		DatabaseIterator { iter: self.db.iterator(IteratorMode::Start) }
	}
}

#[cfg(test)]
mod tests {
	use hash::*;
	use super::*;
	use devtools::*;
	use std::str::FromStr;
	use std::ops::Deref;

	fn test_db(config: &DatabaseConfig) {
		let path = RandomTempPath::create_dir();
		let db = Database::open(config, path.as_path().to_str().unwrap()).unwrap();
		let key1 = H256::from_str("02c69be41d0b7e40352fc85be1cd65eb03d40ef8427a0ca4596b1ead9a00e9fc").unwrap();
		let key2 = H256::from_str("03c69be41d0b7e40352fc85be1cd65eb03d40ef8427a0ca4596b1ead9a00e9fc").unwrap();
		let key3 = H256::from_str("01c69be41d0b7e40352fc85be1cd65eb03d40ef8427a0ca4596b1ead9a00e9fc").unwrap();

		db.put(&key1, b"cat").unwrap();
		db.put(&key2, b"dog").unwrap();

		assert_eq!(db.get(&key1).unwrap().unwrap().deref(), b"cat");

		let contents: Vec<_> = db.iter().collect();
		assert_eq!(contents.len(), 2);
		assert_eq!(&*contents[0].0, key1.deref());
		assert_eq!(&*contents[0].1, b"cat");
		assert_eq!(&*contents[1].0, key2.deref());
		assert_eq!(&*contents[1].1, b"dog");

		db.delete(&key1).unwrap();
		assert!(db.get(&key1).unwrap().is_none());
		db.put(&key1, b"cat").unwrap();

		let transaction = DBTransaction::new();
		transaction.put(&key3, b"elephant").unwrap();
		transaction.delete(&key1).unwrap();
		db.write(transaction).unwrap();
		assert!(db.get(&key1).unwrap().is_none());
		assert_eq!(db.get(&key3).unwrap().unwrap().deref(), b"elephant");

		if config.prefix_size.is_some() {
			assert_eq!(db.get_by_prefix(&key3).unwrap().deref(), b"elephant");
			assert_eq!(db.get_by_prefix(&key2).unwrap().deref(), b"dog");
		}
	}

	#[test]
	fn kvdb() {
		let path = RandomTempPath::create_dir();
		let smoke = Database::open_default(path.as_path().to_str().unwrap()).unwrap();
		assert!(smoke.is_empty());
		test_db(&DatabaseConfig::default());
		test_db(&DatabaseConfig::default().prefix(12));
		test_db(&DatabaseConfig::default().prefix(22));
		test_db(&DatabaseConfig::default().prefix(8));
	}
}

