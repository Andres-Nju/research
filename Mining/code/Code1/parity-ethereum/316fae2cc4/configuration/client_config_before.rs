	pub fn client_config(&self, spec: &Spec) -> ClientConfig {
		let mut client_config = ClientConfig::default();

		match self.args.flag_cache {
			Some(mb) => {
				client_config.blockchain.max_cache_size = mb * 1024 * 1024;
				client_config.blockchain.pref_cache_size = client_config.blockchain.max_cache_size * 3 / 4;
			}
			None => {
				client_config.blockchain.pref_cache_size = self.args.flag_cache_pref_size;
				client_config.blockchain.max_cache_size = self.args.flag_cache_max_size;
			}
		}
		// forced blockchain (blocks + extras) db cache size if provided
		client_config.blockchain.db_cache_size = self.args.flag_db_cache_size.and_then(|cs| Some(cs / 2));

		client_config.tracing.enabled = match self.args.flag_tracing.as_str() {
			"auto" => Switch::Auto,
			"on" => Switch::On,
			"off" => Switch::Off,
			_ => { die!("Invalid tracing method given!") }
		};
		// forced trace db cache size if provided
		client_config.tracing.db_cache_size = self.args.flag_db_cache_size.and_then(|cs| Some(cs / 4));

		client_config.pruning = match self.args.flag_pruning.as_str() {
			"archive" => journaldb::Algorithm::Archive,
			"light" => journaldb::Algorithm::EarlyMerge,
			"fast" => journaldb::Algorithm::OverlayRecent,
			"basic" => journaldb::Algorithm::RefCounted,
			"auto" => self.find_best_db(spec).unwrap_or(journaldb::Algorithm::OverlayRecent),
			_ => { die!("Invalid pruning method given."); }
		};

		if self.args.flag_fat_db {
			if let journaldb::Algorithm::Archive = client_config.pruning {
				client_config.trie_spec = TrieSpec::Fat;
			} else {
				die!("Fatdb is not supported. Please rerun with --pruning=archive")
			}
		}

		// forced state db cache size if provided
		client_config.db_cache_size = self.args.flag_db_cache_size.and_then(|cs| Some(cs / 4));

		// compaction profile
		client_config.db_compaction = match self.args.flag_db_compaction.as_str() {
			"ssd" => DatabaseCompactionProfile::Default,
			"hdd" => DatabaseCompactionProfile::HDD,
			_ => { die!("Invalid compaction profile given (--db-compaction argument), expected hdd/default."); }
		};

		if self.args.flag_jitvm {
			client_config.vm_type = VMType::jit().unwrap_or_else(|| die!("Parity built without jit vm."))
		}

		trace!(target: "parity", "Using pruning strategy of {}", client_config.pruning);
		client_config.name = self.args.flag_identity.clone();
		client_config.queue.max_mem_use = self.args.flag_queue_max_size;
		client_config
	}
