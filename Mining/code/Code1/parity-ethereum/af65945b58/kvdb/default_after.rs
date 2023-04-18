	fn default() -> DatabaseConfig {
		DatabaseConfig {
			cache_size: None,
			prefix_size: None,
			max_open_files: 256,
			compaction: CompactionProfile::default(),
		}
	}
