	pub fn with_cache(cache_size: usize) -> DatabaseConfig {
		DatabaseConfig {
			cache_size: Some(cache_size),
			prefix_size: None,
			max_open_files: -1,
			compaction: CompactionProfile::default(),
		}
	}
