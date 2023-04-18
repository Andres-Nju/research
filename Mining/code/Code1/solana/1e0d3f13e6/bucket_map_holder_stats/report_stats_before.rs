    pub fn report_stats<T: IndexValue>(&self, storage: &BucketMapHolder<T>) {
        let elapsed_ms = self.last_time.elapsed_ms();
        if elapsed_ms < STATS_INTERVAL_MS {
            return;
        }

        if !self.last_time.should_update(STATS_INTERVAL_MS) {
            return;
        }

        let ms_per_age = self.ms_per_age(storage, elapsed_ms);

        let in_mem_per_bucket_counts = self
            .per_bucket_count
            .iter()
            .map(|count| count.load(Ordering::Relaxed))
            .collect::<Vec<_>>();
        let disk = storage.disk.as_ref();
        let disk_per_bucket_counts = disk
            .map(|disk| {
                disk.stats
                    .per_bucket_count
                    .iter()
                    .map(|count| count.load(Ordering::Relaxed) as usize)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let in_mem_stats = Self::get_stats(in_mem_per_bucket_counts);
        let disk_stats = Self::get_stats(disk_per_bucket_counts);

        const US_PER_MS: u64 = 1_000;

        // all metrics during startup are written to a different data point
        let startup = storage.get_startup();
        let was_startup = self.last_was_startup.swap(startup, Ordering::Relaxed);

        // sum of elapsed time in each thread
        let mut thread_time_elapsed_ms = elapsed_ms * storage.threads as u64;
        if disk.is_some() {
            datapoint_info!(
                if startup || was_startup {
                    thread_time_elapsed_ms *= 2; // more threads are allocated during startup
                    "accounts_index_startup"
                } else {
                    "accounts_index"
                },
                (
                    "count_in_mem",
                    self.count_in_mem.load(Ordering::Relaxed),
                    i64
                ),
                ("count", self.total_count(), i64),
                (
                    "bg_waiting_percent",
                    Self::calc_percent(
                        self.bg_waiting_us.swap(0, Ordering::Relaxed) / US_PER_MS,
                        thread_time_elapsed_ms
                    ),
                    f64
                ),
                (
                    "bg_throttling_wait_percent",
                    Self::calc_percent(
                        self.bg_throttling_wait_us.swap(0, Ordering::Relaxed) / US_PER_MS,
                        thread_time_elapsed_ms
                    ),
                    f64
                ),
                (
                    "held_in_mem_slot_list_len",
                    self.held_in_mem_slot_list_len.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "held_in_mem_slot_list_cached",
                    self.held_in_mem_slot_list_cached.swap(0, Ordering::Relaxed),
                    i64
                ),
                ("min_in_bin_mem", in_mem_stats.0, i64),
                ("max_in_bin_mem", in_mem_stats.1, i64),
                ("count_from_bins_mem", in_mem_stats.2, i64),
                ("median_from_bins_mem", in_mem_stats.3, i64),
                ("min_in_bin_disk", disk_stats.0, i64),
                ("max_in_bin_disk", disk_stats.1, i64),
                ("count_from_bins_disk", disk_stats.2, i64),
                ("median_from_bins_disk", disk_stats.3, i64),
                (
                    "gets_from_mem",
                    self.gets_from_mem.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "get_mem_us",
                    self.get_mem_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "gets_missing",
                    self.gets_missing.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "get_missing_us",
                    self.get_missing_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "entries_from_mem",
                    self.entries_from_mem.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "entry_mem_us",
                    self.entry_mem_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "load_disk_found_count",
                    self.load_disk_found_count.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "load_disk_found_us",
                    self.load_disk_found_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "load_disk_missing_count",
                    self.load_disk_missing_count.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "load_disk_missing_us",
                    self.load_disk_missing_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "entries_missing",
                    self.entries_missing.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "entry_missing_us",
                    self.entry_missing_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "updates_in_mem",
                    self.updates_in_mem.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "get_range_us",
                    self.get_range_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                ("inserts", self.inserts.swap(0, Ordering::Relaxed), i64),
                ("deletes", self.deletes.swap(0, Ordering::Relaxed), i64),
                (
                    "active_threads",
                    self.active_threads.load(Ordering::Relaxed),
                    i64
                ),
                ("items", self.items.swap(0, Ordering::Relaxed), i64),
                ("keys", self.keys.swap(0, Ordering::Relaxed), i64),
                ("ms_per_age", ms_per_age, i64),
                (
                    "flush_scan_update_us",
                    self.flush_scan_update_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "flush_grow_us",
                    self.flush_remove_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "flush_remove_us",
                    self.flush_remove_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "disk_index_resizes",
                    disk.map(|disk| disk.stats.index.resizes.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_index_max_size",
                    disk.map(|disk| {
                        let mut lock = disk.stats.index.max_size.lock().unwrap();
                        let value = *lock;
                        *lock = 0;
                        value
                    })
                    .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_index_new_file_us",
                    disk.map(|disk| disk.stats.index.new_file_us.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_index_resize_us",
                    disk.map(|disk| disk.stats.index.resize_us.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_index_flush_file_us",
                    disk.map(|disk| disk.stats.index.flush_file_us.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_index_flush_mmap_us",
                    disk.map(|disk| disk.stats.index.mmap_us.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_data_resizes",
                    disk.map(|disk| disk.stats.data.resizes.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_data_max_size",
                    disk.map(|disk| {
                        let mut lock = disk.stats.data.max_size.lock().unwrap();
                        let value = *lock;
                        *lock = 0;
                        value
                    })
                    .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_data_new_file_us",
                    disk.map(|disk| disk.stats.data.new_file_us.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_data_resize_us",
                    disk.map(|disk| disk.stats.data.resize_us.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_data_flush_file_us",
                    disk.map(|disk| disk.stats.data.flush_file_us.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "disk_data_flush_mmap_us",
                    disk.map(|disk| disk.stats.data.mmap_us.swap(0, Ordering::Relaxed))
                        .unwrap_or_default(),
                    i64
                ),
                (
                    "flush_entries_updated_on_disk",
                    self.flush_entries_updated_on_disk
                        .swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "flush_entries_removed_from_mem",
                    self.flush_entries_removed_from_mem
                        .swap(0, Ordering::Relaxed),
                    i64
                ),
            );
        } else {
            datapoint_info!(
                if startup || was_startup {
                    thread_time_elapsed_ms *= 2; // more threads are allocated during startup
                    "accounts_index_startup"
                } else {
                    "accounts_index"
                },
                (
                    "count_in_mem",
                    self.count_in_mem.load(Ordering::Relaxed),
                    i64
                ),
                ("count", self.total_count(), i64),
                (
                    "bg_waiting_percent",
                    Self::calc_percent(
                        self.bg_waiting_us.swap(0, Ordering::Relaxed) / US_PER_MS,
                        thread_time_elapsed_ms
                    ),
                    f64
                ),
                (
                    "bg_throttling_wait_percent",
                    Self::calc_percent(
                        self.bg_throttling_wait_us.swap(0, Ordering::Relaxed) / US_PER_MS,
                        thread_time_elapsed_ms
                    ),
                    f64
                ),
                ("min_in_bin_mem", in_mem_stats.0, i64),
                ("max_in_bin_mem", in_mem_stats.1, i64),
                ("count_from_bins_mem", in_mem_stats.2, i64),
                ("median_from_bins_mem", in_mem_stats.3, i64),
                (
                    "gets_from_mem",
                    self.gets_from_mem.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "get_mem_us",
                    self.get_mem_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "gets_missing",
                    self.gets_missing.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "get_missing_us",
                    self.get_missing_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "entries_from_mem",
                    self.entries_from_mem.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "entry_mem_us",
                    self.entry_mem_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "entries_missing",
                    self.entries_missing.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "entry_missing_us",
                    self.entry_missing_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "updates_in_mem",
                    self.updates_in_mem.swap(0, Ordering::Relaxed),
                    i64
                ),
                (
                    "get_range_us",
                    self.get_range_us.swap(0, Ordering::Relaxed),
                    i64
                ),
                ("inserts", self.inserts.swap(0, Ordering::Relaxed), i64),
                ("deletes", self.deletes.swap(0, Ordering::Relaxed), i64),
                (
                    "active_threads",
                    self.active_threads.load(Ordering::Relaxed),
                    i64
                ),
                ("items", self.items.swap(0, Ordering::Relaxed), i64),
                ("keys", self.keys.swap(0, Ordering::Relaxed), i64),
            );
        }
    }
