	pub fn with_cache(cache_size: usize) -> DatabaseConfig {
		DatabaseConfig {
			cache_size: Some(cache_size),
			prefix_size: None,
			max_open_files: 256,
			compaction: CompactionProfile::default(),
		}
	}
