	fn default() -> DatabaseConfig {
		DatabaseConfig {
			cache_size: None,
			prefix_size: None,
			max_open_files: -1,
			compaction: CompactionProfile::default(),
		}
	}
