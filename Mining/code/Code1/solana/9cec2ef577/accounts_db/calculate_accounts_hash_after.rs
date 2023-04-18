    pub fn calculate_accounts_hash(
        &self,
        max_slot: Slot,
        config: &CalcAccountsHashConfig<'_>,
    ) -> Result<(Hash, u64), BankHashVerificationError> {
        use BankHashVerificationError::*;
        let mut collect = Measure::start("collect");
        let keys: Vec<_> = self
            .accounts_index
            .account_maps
            .iter()
            .flat_map(|map| {
                let mut keys = map.keys();
                keys.sort_unstable(); // hashmap is not ordered, but bins are relative to each other
                keys
            })
            .collect();
        collect.stop();

        let mut scan = Measure::start("scan");
        let mismatch_found = AtomicU64::new(0);
        // Pick a chunk size big enough to allow us to produce output vectors that are smaller than the overall size.
        // We'll also accumulate the lamports within each chunk and fewer chunks results in less contention to accumulate the sum.
        let chunks = crate::accounts_hash::MERKLE_FANOUT.pow(4);
        let total_lamports = Mutex::<u64>::new(0);
        let stats = HashStats::default();

        let max_slot_info = SlotInfoInEpoch::new(max_slot, config.epoch_schedule);

        let get_hashes = || {
            keys.par_chunks(chunks)
                .map(|pubkeys| {
                    let mut sum = 0u128;
                    let result: Vec<Hash> = pubkeys
                        .iter()
                        .filter_map(|pubkey| {
                            if self.is_filler_account(pubkey) {
                                return None;
                            }
                            if let AccountIndexGetResult::Found(lock, index) =
                                self.accounts_index.get(pubkey, config.ancestors, Some(max_slot))
                            {
                                let (slot, account_info) = &lock.slot_list()[index];
                                if !account_info.is_zero_lamport() {
                                    // Because we're keeping the `lock' here, there is no need
                                    // to use retry_to_get_account_accessor()
                                    // In other words, flusher/shrinker/cleaner is blocked to
                                    // cause any Accessor(None) situation.
                                    // Anyway this race condition concern is currently a moot
                                    // point because calculate_accounts_hash() should not
                                    // currently race with clean/shrink because the full hash
                                    // is synchronous with clean/shrink in
                                    // AccountsBackgroundService
                                    self.get_account_accessor(
                                        *slot,
                                        pubkey,
                                        &account_info.storage_location(),
                                    )
                                    .get_loaded_account()
                                    .and_then(
                                        |loaded_account| {
                                            let find_unskipped_slot = |slot: Slot| {
                                                self.find_unskipped_slot(slot, config.ancestors)
                                            };
                                            let loaded_hash = loaded_account.loaded_hash();
                                            let new_hash = config.enable_rehashing
                                                .then(|| ExpectedRentCollection::maybe_rehash_skipped_rewrite(
                                                    &loaded_account,
                                                    &loaded_hash,
                                                    pubkey,
                                                    *slot,
                                                    config.epoch_schedule,
                                                    config.rent_collector,
                                                    &stats,
                                                    &max_slot_info,
                                                    find_unskipped_slot,
                                                    self.filler_account_suffix.as_ref(),
                                                )).flatten();
                                            let loaded_hash = new_hash.unwrap_or(loaded_hash);
                                            let balance = loaded_account.lamports();
                                            if config.check_hash && !self.is_filler_account(pubkey) {  // this will not be supported anymore
                                                let computed_hash =
                                                    loaded_account.compute_hash(*slot, pubkey);
                                                if computed_hash != loaded_hash {
                                                    info!("hash mismatch found: computed: {}, loaded: {}, pubkey: {}", computed_hash, loaded_hash, pubkey);
                                                    mismatch_found
                                                        .fetch_add(1, Ordering::Relaxed);
                                                    return None;
                                                }
                                            }

                                            sum += balance as u128;
                                            Some(loaded_hash)
                                        },
                                    )
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();
                    let mut total = total_lamports.lock().unwrap();
                    *total =
                        AccountsHash::checked_cast_for_capitalization(*total as u128 + sum);
                    result
                }).collect()
        };

        let hashes: Vec<Vec<Hash>> = if config.check_hash {
            get_hashes()
        } else {
            self.thread_pool_clean.install(get_hashes)
        };
        if mismatch_found.load(Ordering::Relaxed) > 0 {
            warn!(
                "{} mismatched account hash(es) found",
                mismatch_found.load(Ordering::Relaxed)
            );
            return Err(MismatchedAccountHash);
        }

        scan.stop();
        let total_lamports = *total_lamports.lock().unwrap();

        let mut hash_time = Measure::start("hash");
        let (accumulated_hash, hash_total) = AccountsHash::calculate_hash(hashes);
        hash_time.stop();
        datapoint_info!(
            "update_accounts_hash",
            ("accounts_scan", scan.as_us(), i64),
            ("hash", hash_time.as_us(), i64),
            ("hash_total", hash_total, i64),
            ("collect", collect.as_us(), i64),
            (
                "rehashed_rewrites",
                stats.rehash_required.load(Ordering::Relaxed),
                i64
            ),
            (
                "rehashed_rewrites_unnecessary",
                stats.rehash_unnecessary.load(Ordering::Relaxed),
                i64
            ),
        );
        self.assert_safe_squashing_accounts_hash(max_slot, config.epoch_schedule);

        Ok((accumulated_hash, total_lamports))
    }

    pub fn get_accounts_hash(&self, slot: Slot) -> Hash {
        let bank_hashes = self.bank_hashes.read().unwrap();
        let bank_hash_info = bank_hashes.get(&slot).unwrap();
        bank_hash_info.snapshot_hash
    }

    pub fn update_accounts_hash(
        &self,
        slot: Slot,
        ancestors: &Ancestors,
        epoch_schedule: &EpochSchedule,
        rent_collector: &RentCollector,
        enable_rehashing: bool,
    ) -> (Hash, u64) {
        self.update_accounts_hash_with_index_option(
            true,
            false,
            slot,
            ancestors,
            None,
            false,
            epoch_schedule,
            rent_collector,
            false,
            enable_rehashing,
        )
    }

    #[cfg(test)]
    fn update_accounts_hash_test(&self, slot: Slot, ancestors: &Ancestors) -> (Hash, u64) {
        self.update_accounts_hash_with_index_option(
            true,
            true,
            slot,
            ancestors,
            None,
            false,
            &EpochSchedule::default(),
            &RentCollector::default(),
            false,
            true,
        )
    }

    fn scan_multiple_account_storages_one_slot<S>(
        storages: &[Arc<AccountStorageEntry>],
        scanner: &mut S,
    ) where
        S: AppendVecScan,
    {
        let mut len = storages.len();
        if len == 1 {
            // only 1 storage, so no need to interleave between multiple storages based on write_version
            storages[0].accounts.account_iter().for_each(|account| {
                if scanner.filter(&account.meta.pubkey) {
                    scanner.found_account(&LoadedAccount::Stored(account))
                }
            });
        } else {
            // we have to call the scan_func in order of write_version within a slot if there are multiple storages per slot
            let mut progress = Vec::with_capacity(len);
            let mut current =
                Vec::<(StoredMetaWriteVersion, Option<StoredAccountMeta<'_>>)>::with_capacity(len);
            for storage in storages {
                let mut iterator = storage.accounts.account_iter();
                if let Some(item) = iterator
                    .next()
                    .map(|stored_account| (stored_account.meta.write_version, Some(stored_account)))
                {
                    current.push(item);
                    progress.push(iterator);
                }
            }
            while !progress.is_empty() {
                let mut min = current[0].0;
                let mut min_index = 0;
                for (i, (item, _)) in current.iter().enumerate().take(len).skip(1) {
                    if item < &min {
                        min_index = i;
                        min = *item;
                    }
                }
                let found_account = &mut current[min_index];
                if scanner.filter(
                    &found_account
                        .1
                        .as_ref()
                        .map(|stored_account| stored_account.meta.pubkey)
                        .unwrap(), // will always be 'Some'
                ) {
                    let account = std::mem::take(found_account);
                    scanner.found_account(&LoadedAccount::Stored(account.1.unwrap()));
                }
                let next = progress[min_index].next().map(|stored_account| {
                    (stored_account.meta.write_version, Some(stored_account))
                });
                match next {
                    Some(item) => {
                        current[min_index] = item;
                    }
                    None => {
                        current.remove(min_index);
                        progress.remove(min_index);
                        len -= 1;
                    }
                }
            }
        }
    }

    fn update_old_slot_stats(
        &self,
        stats: &HashStats,
        sub_storages: Option<&Vec<Arc<AccountStorageEntry>>>,
    ) {
        if let Some(sub_storages) = sub_storages {
            stats.roots_older_than_epoch.fetch_add(1, Ordering::Relaxed);
            let mut ancients = 0;
            let num_accounts = sub_storages
                .iter()
                .map(|storage| {
                    if is_ancient(&storage.accounts) {
                        ancients += 1;
                    }
                    storage.count()
                })
                .sum();
            let sizes = sub_storages
                .iter()
                .map(|storage| storage.total_bytes())
                .sum::<u64>();
            stats
                .append_vec_sizes_older_than_epoch
                .fetch_add(sizes as usize, Ordering::Relaxed);
            stats
                .accounts_in_roots_older_than_epoch
                .fetch_add(num_accounts, Ordering::Relaxed);
            stats
                .ancient_append_vecs
                .fetch_add(ancients, Ordering::Relaxed);
        }
    }

    /// Scan through all the account storage in parallel
    fn scan_account_storage_no_bank<S>(
        &self,
        cache_hash_data: &CacheHashData,
        config: &CalcAccountsHashConfig<'_>,
        snapshot_storages: &SortedStorages,
        scanner: S,
        bin_range: &Range<usize>,
        bin_calculator: &PubkeyBinCalculator24,
        stats: &HashStats,
    ) -> Vec<BinnedHashData>
    where
        S: AppendVecScan,
    {
        let start_bin_index = bin_range.start;

        // any ancient append vecs should definitely be cached
        // We need to break the ranges into:
        // 1. individual ancient append vecs (may be empty)
        // 2. first unevenly divided chunk starting at 1 epoch old slot (may be empty)
        // 3. evenly divided full chunks in the middle
        // 4. unevenly divided chunk of most recent slots (may be empty)
        let max_slot_inclusive = snapshot_storages.max_slot_inclusive();
        // we are going to use a fixed slots per epoch here.
        // We are mainly interested in the network at steady state.
        let slots_in_epoch = config.epoch_schedule.slots_per_epoch;
        let one_epoch_old_slot = max_slot_inclusive.saturating_sub(slots_in_epoch);

        let range = snapshot_storages.range();
        let ancient_slots = snapshot_storages
            .iter_range(range.start..one_epoch_old_slot)
            .filter_map(|(slot, storages)| storages.map(|_| slot))
            .collect::<Vec<_>>();
        let ancient_slot_count = ancient_slots.len() as Slot;
        let slot0 = std::cmp::max(range.start, one_epoch_old_slot);
        let first_boundary =
            ((slot0 + MAX_ITEMS_PER_CHUNK) / MAX_ITEMS_PER_CHUNK) * MAX_ITEMS_PER_CHUNK;

        let width = max_slot_inclusive - slot0;
        // 2 is for 2 special chunks - unaligned slots at the beginning and end
        let chunks = ancient_slot_count + 2 + (width as Slot / MAX_ITEMS_PER_CHUNK);
        (0..chunks)
            .into_par_iter()
            .map(|mut chunk| {
                let mut scanner = scanner.clone();
                // calculate start, end_exclusive
                let (single_cached_slot, (start, mut end_exclusive)) = if chunk < ancient_slot_count
                {
                    let ancient_slot = ancient_slots[chunk as usize];
                    (true, (ancient_slot, ancient_slot + 1))
                } else {
                    (false, {
                        chunk -= ancient_slot_count;
                        if chunk == 0 {
                            if slot0 == first_boundary {
                                return scanner.scanning_complete(); // if we evenly divide, nothing for special chunk 0 to do
                            }
                            // otherwise first chunk is not 'full'
                            (slot0, first_boundary)
                        } else {
                            // normal chunk in the middle or at the end
                            let start = first_boundary + MAX_ITEMS_PER_CHUNK * (chunk - 1);
                            let end_exclusive = start + MAX_ITEMS_PER_CHUNK;
                            (start, end_exclusive)
                        }
                    })
                };
                end_exclusive = std::cmp::min(end_exclusive, range.end);
                if start == end_exclusive {
                    return scanner.scanning_complete();
                }

                let should_cache_hash_data = CalcAccountsHashConfig::get_should_cache_hash_data()
                    || config.store_detailed_debug_info_on_failure;

                // if we're using the write cache, then we can't rely on cached append vecs since the append vecs may not include every account
                // Single cached slots get cached and full chunks get cached.
                // chunks that don't divide evenly would include some cached append vecs that are no longer part of this range and some that are, so we have to ignore caching on non-evenly dividing chunks.
                let eligible_for_caching = !config.use_write_cache
                    && (single_cached_slot
                        || end_exclusive.saturating_sub(start) == MAX_ITEMS_PER_CHUNK);

                if eligible_for_caching || config.store_detailed_debug_info_on_failure {
                    let range = bin_range.end - bin_range.start;
                    scanner.init_accum(range);
                }

                let slots_per_epoch = config
                    .rent_collector
                    .epoch_schedule
                    .get_slots_in_epoch(config.rent_collector.epoch);
                let one_epoch_old = snapshot_storages
                    .range()
                    .end
                    .saturating_sub(slots_per_epoch);

                let mut file_name = String::default();
                // if we're using the write cache, we can't cache the hash calc results because not all accounts are in append vecs.
                if (should_cache_hash_data && eligible_for_caching)
                    || config.store_detailed_debug_info_on_failure
                {
                    let mut load_from_cache = true;
                    let mut hasher = std::collections::hash_map::DefaultHasher::new(); // wrong one?

                    for (slot, sub_storages) in snapshot_storages.iter_range(start..end_exclusive) {
                        if bin_range.start == 0 && slot < one_epoch_old {
                            self.update_old_slot_stats(stats, sub_storages);
                        }
                        bin_range.start.hash(&mut hasher);
                        bin_range.end.hash(&mut hasher);
                        if let Some(sub_storages) = sub_storages {
                            if sub_storages.len() > 1
                                && !config.store_detailed_debug_info_on_failure
                            {
                                // Having > 1 appendvecs per slot is not expected. If we have that, we just fail to cache this slot.
                                // However, if we're just dumping detailed debug info, we don't care, so store anyway.
                                load_from_cache = false;
                                break;
                            }
                            let storage_file = sub_storages.first().unwrap().accounts.get_path();
                            slot.hash(&mut hasher);
                            storage_file.hash(&mut hasher);
                            // check alive_bytes, etc. here?
                            let amod = std::fs::metadata(storage_file);
                            if amod.is_err() {
                                load_from_cache = false;
                                break;
                            }
                            let amod = amod.unwrap().modified();
                            if amod.is_err() {
                                load_from_cache = false;
                                break;
                            }
                            let amod = amod
                                .unwrap()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            amod.hash(&mut hasher);
                        }
                    }
                    if load_from_cache {
                        // we have a hash value for all the storages in this slot
                        // so, build a file name:
                        let hash = hasher.finish();
                        file_name = format!(
                            "{}.{}.{}.{}.{}",
                            start, end_exclusive, bin_range.start, bin_range.end, hash
                        );
                        let mut retval = scanner.get_accum();
                        if eligible_for_caching
                            && cache_hash_data
                                .load(
                                    &Path::new(&file_name),
                                    &mut retval,
                                    start_bin_index,
                                    bin_calculator,
                                )
                                .is_ok()
                        {
                            return retval;
                        }
                        scanner.set_accum(retval);

                        // fall through and load normally - we failed to load
                    }
                } else {
                    for (slot, sub_storages) in snapshot_storages.iter_range(start..end_exclusive) {
                        if bin_range.start == 0 && slot < one_epoch_old {
                            self.update_old_slot_stats(stats, sub_storages);
                        }
                    }
                }

                for (slot, sub_storages) in snapshot_storages.iter_range(start..end_exclusive) {
                    scanner.set_slot(slot);
                    let valid_slot = sub_storages.is_some();
                    if config.use_write_cache {
                        let ancestors = config.ancestors.as_ref().unwrap();
                        if let Some(slot_cache) = self.accounts_cache.slot_cache(slot) {
                            if valid_slot
                                || ancestors.contains_key(&slot)
                                || self.accounts_index.is_alive_root(slot)
                            {
                                let keys = slot_cache.get_all_pubkeys();
                                for key in keys {
                                    if scanner.filter(&key) {
                                        if let Some(cached_account) = slot_cache.get_cloned(&key) {
                                            let mut accessor = LoadedAccountAccessor::Cached(Some(
                                                Cow::Owned(cached_account),
                                            ));
                                            let account = accessor.get_loaded_account().unwrap();
                                            scanner.found_account(&account);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(sub_storages) = sub_storages {
                        Self::scan_multiple_account_storages_one_slot(sub_storages, &mut scanner);
                    }
                }
                let r = scanner.scanning_complete();
                if !file_name.is_empty() {
                    let result = cache_hash_data.save(Path::new(&file_name), &r);

                    if result.is_err() {
                        info!(
                            "FAILED_TO_SAVE: {}-{}, {}, first_boundary: {}, {:?}, error: {:?}",
                            range.start, range.end, width, first_boundary, file_name, result,
                        );
                    }
                }
                r
            })
            .filter(|x| !x.is_empty())
            .collect()
    }

    /// storages are sorted by slot and have range info.
    /// add all stores older than slots_per_epoch to dirty_stores so clean visits these slots
    fn mark_old_slots_as_dirty(
        &self,
        storages: &SortedStorages,
        slots_per_epoch: Slot,
        mut stats: &mut crate::accounts_hash::HashStats,
    ) {
        let mut mark_time = Measure::start("mark_time");
        let mut num_dirty_slots: usize = 0;
        let max = storages.max_slot_inclusive();
        let acceptable_straggler_slot_count = 100; // do nothing special for these old stores which will likely get cleaned up shortly
        let sub = slots_per_epoch + acceptable_straggler_slot_count;
        let in_epoch_range_start = max.saturating_sub(sub);
        for (slot, storages) in storages.iter_range(..in_epoch_range_start) {
            if let Some(storages) = storages {
                storages.iter().for_each(|store| {
                    if !is_ancient(&store.accounts) {
                        // ancient stores are managed separately - we expect them to be old and keeping accounts
                        // We can expect the normal processes will keep them cleaned.
                        // If we included them here then ALL accounts in ALL ancient append vecs will be visited by clean each time.
                        self.dirty_stores
                            .insert((slot, store.append_vec_id()), store.clone());
                        num_dirty_slots += 1;
                    }
                });
            }
        }
        mark_time.stop();
        stats.mark_time_us = mark_time.as_us();
        stats.num_dirty_slots = num_dirty_slots;
    }

    pub(crate) fn calculate_accounts_hash_helper(
        &self,
        use_index: bool,
        slot: Slot,
        config: &CalcAccountsHashConfig<'_>,
    ) -> Result<(Hash, u64), BankHashVerificationError> {
        if !use_index {
            let mut collect_time = Measure::start("collect");
            let (combined_maps, slots) = self.get_snapshot_storages(slot, None, config.ancestors);
            collect_time.stop();

            let mut sort_time = Measure::start("sort_storages");
            let min_root = self.accounts_index.min_alive_root();
            let storages = SortedStorages::new_with_slots(
                combined_maps.iter().zip(slots.into_iter()),
                min_root,
                Some(slot),
            );
            sort_time.stop();

            let mut timings = HashStats {
                collect_snapshots_us: collect_time.as_us(),
                storage_sort_us: sort_time.as_us(),
                ..HashStats::default()
            };
            timings.calc_storage_size_quartiles(&combined_maps);

            self.calculate_accounts_hash_without_index(config, &storages, timings)
        } else {
            self.calculate_accounts_hash(slot, config)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn calculate_accounts_hash_helper_with_verify(
        &self,
        use_index: bool,
        debug_verify: bool,
        slot: Slot,
        config: CalcAccountsHashConfig<'_>,
        expected_capitalization: Option<u64>,
    ) -> Result<(Hash, u64), BankHashVerificationError> {
        let (hash, total_lamports) =
            self.calculate_accounts_hash_helper(use_index, slot, &config)?;
        if debug_verify {
            // calculate the other way (store or non-store) and verify results match.
            let (hash_other, total_lamports_other) =
                self.calculate_accounts_hash_helper(!use_index, slot, &config)?;

            let success = hash == hash_other
                && total_lamports == total_lamports_other
                && total_lamports == expected_capitalization.unwrap_or(total_lamports);
            assert!(success, "update_accounts_hash_with_index_option mismatch. hashes: {}, {}; lamports: {}, {}; expected lamports: {:?}, using index: {}, slot: {}", hash, hash_other, total_lamports, total_lamports_other, expected_capitalization, use_index, slot);
        }
        Ok((hash, total_lamports))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_accounts_hash_with_index_option(
        &self,
        use_index: bool,
        debug_verify: bool,
        slot: Slot,
        ancestors: &Ancestors,
        expected_capitalization: Option<u64>,
        can_cached_slot_be_unflushed: bool,
        epoch_schedule: &EpochSchedule,
        rent_collector: &RentCollector,
        is_startup: bool,
        enable_rehashing: bool,
    ) -> (Hash, u64) {
        let check_hash = false;
        let (hash, total_lamports) = self
            .calculate_accounts_hash_helper_with_verify(
                use_index,
                debug_verify,
                slot,
                CalcAccountsHashConfig {
                    use_bg_thread_pool: !is_startup,
                    check_hash,
                    ancestors: Some(ancestors),
                    use_write_cache: can_cached_slot_be_unflushed,
                    epoch_schedule,
                    rent_collector,
                    store_detailed_debug_info_on_failure: false,
                    full_snapshot: None,
                    enable_rehashing,
                },
                expected_capitalization,
            )
            .unwrap(); // unwrap here will never fail since check_hash = false
        self.set_accounts_hash(slot, hash);
        (hash, total_lamports)
    }

    /// update hash for this slot in the 'bank_hashes' map
    pub(crate) fn set_accounts_hash(&self, slot: Slot, hash: Hash) {
        let mut bank_hashes = self.bank_hashes.write().unwrap();
        let mut bank_hash_info = bank_hashes.get_mut(&slot).unwrap();
        bank_hash_info.snapshot_hash = hash;
    }

    fn scan_snapshot_stores_with_cache(
        &self,
        cache_hash_data: &CacheHashData,
        storage: &SortedStorages,
        mut stats: &mut crate::accounts_hash::HashStats,
        bins: usize,
        bin_range: &Range<usize>,
        config: &CalcAccountsHashConfig<'_>,
        filler_account_suffix: Option<&Pubkey>,
    ) -> Result<Vec<BinnedHashData>, BankHashVerificationError> {
        let bin_calculator = PubkeyBinCalculator24::new(bins);
        assert!(bin_range.start < bins && bin_range.end <= bins && bin_range.start < bin_range.end);
        let mut time = Measure::start("scan all accounts");
        stats.num_snapshot_storage = storage.storage_count();
        stats.num_slots = storage.slot_count();
        let mismatch_found = Arc::new(AtomicU64::new(0));
        let range = bin_range.end - bin_range.start;
        let sort_time = Arc::new(AtomicU64::new(0));

        let find_unskipped_slot = |slot: Slot| self.find_unskipped_slot(slot, config.ancestors);

        let max_slot_info =
            SlotInfoInEpoch::new(storage.max_slot_inclusive(), config.epoch_schedule);
        let scanner = ScanState {
            current_slot: Slot::default(),
            accum: BinnedHashData::default(),
            bin_calculator: &bin_calculator,
            config,
            mismatch_found: mismatch_found.clone(),
            max_slot_info,
            find_unskipped_slot: &find_unskipped_slot,
            filler_account_suffix,
            range,
            bin_range,
            stats,
            sort_time: sort_time.clone(),
            pubkey_to_bin_index: 0,
        };

        let result: Vec<BinnedHashData> = self.scan_account_storage_no_bank(
            cache_hash_data,
            config,
            storage,
            scanner,
            bin_range,
            &bin_calculator,
            stats,
        );

        stats.sort_time_total_us += sort_time.load(Ordering::Relaxed);

        if config.check_hash && mismatch_found.load(Ordering::Relaxed) > 0 {
            warn!(
                "{} mismatched account hash(es) found",
                mismatch_found.load(Ordering::Relaxed)
            );
            return Err(BankHashVerificationError::MismatchedAccountHash);
        }

        time.stop();
        stats.scan_time_total_us += time.as_us();

        Ok(result)
    }

    fn sort_slot_storage_scan(accum: BinnedHashData) -> (BinnedHashData, u64) {
        let time = AtomicU64::new(0);
        (
            accum
                .into_iter()
                .map(|mut items| {
                    let mut sort_time = Measure::start("sort");
                    {
                        // sort_by vs unstable because slot and write_version are already in order
                        items.sort_by(AccountsHash::compare_two_hash_entries);
                    }
                    sort_time.stop();
                    time.fetch_add(sort_time.as_us(), Ordering::Relaxed);
                    items
                })
                .collect(),
            time.load(Ordering::Relaxed),
        )
    }

    /// if we ever try to calc hash where there are squashed append vecs within the last epoch, we will fail
    fn assert_safe_squashing_accounts_hash(&self, slot: Slot, epoch_schedule: &EpochSchedule) {
        let previous = self.get_accounts_hash_complete_one_epoch_old();
        let current = Self::get_slot_one_epoch_prior(slot, epoch_schedule);
        assert!(
            previous <= current,
            "get_accounts_hash_complete_one_epoch_old: {}, get_slot_one_epoch_prior: {}, slot: {}",
            previous,
            current,
            slot
        );
    }

    /// normal code path returns the common cache path
    /// when called after a failure has been detected, redirect the cache storage to a separate folder for debugging later
    fn get_cache_hash_data(
        &self,
        config: &CalcAccountsHashConfig<'_>,
        slot: Slot,
    ) -> CacheHashData {
        if !config.store_detailed_debug_info_on_failure {
            CacheHashData::new(&self.accounts_hash_cache_path)
        } else {
            // this path executes when we are failing with a hash mismatch
            let mut new = self.accounts_hash_cache_path.clone();
            new.push("failed_calculate_accounts_hash_cache");
            new.push(slot.to_string());
            let _ = std::fs::remove_dir_all(&new);
            CacheHashData::new(&new)
        }
    }

    // modeled after get_accounts_delta_hash
    // intended to be faster than calculate_accounts_hash
    pub fn calculate_accounts_hash_without_index(
        &self,
        config: &CalcAccountsHashConfig<'_>,
        storages: &SortedStorages<'_>,
        mut stats: HashStats,
    ) -> Result<(Hash, u64), BankHashVerificationError> {
        let _guard = self.active_stats.activate(ActiveStatItem::Hash);
        stats.oldest_root = storages.range().start;

        assert!(
            !(config.store_detailed_debug_info_on_failure && config.use_write_cache),
            "cannot accurately capture all data for debugging if accounts cache is being used"
        );

        self.mark_old_slots_as_dirty(storages, config.epoch_schedule.slots_per_epoch, &mut stats);

        let (num_hash_scan_passes, bins_per_pass) = Self::bins_per_pass(self.num_hash_scan_passes);
        let use_bg_thread_pool = config.use_bg_thread_pool;
        let mut scan_and_hash = || {
            let mut previous_pass = PreviousPass::default();
            let mut final_result = (Hash::default(), 0);

            let cache_hash_data = self.get_cache_hash_data(config, storages.max_slot_inclusive());

            for pass in 0..num_hash_scan_passes {
                let bounds = Range {
                    start: pass * bins_per_pass,
                    end: (pass + 1) * bins_per_pass,
                };

                let hash = AccountsHash {
                    filler_account_suffix: if self.filler_accounts_config.count > 0 {
                        self.filler_account_suffix
                    } else {
                        None
                    },
                };

                let result = self.scan_snapshot_stores_with_cache(
                    &cache_hash_data,
                    storages,
                    &mut stats,
                    PUBKEY_BINS_FOR_CALCULATING_HASHES,
                    &bounds,
                    config,
                    hash.filler_account_suffix.as_ref(),
                )?;

                let (hash, lamports, for_next_pass) = hash.rest_of_hash_calculation(
                    result,
                    &mut stats,
                    pass == num_hash_scan_passes - 1,
                    previous_pass,
                    bins_per_pass,
                );
                previous_pass = for_next_pass;
                final_result = (hash, lamports);
            }

            info!(
                "calculate_accounts_hash_without_index: slot: {} {:?}",
                storages.max_slot_inclusive(),
                final_result
            );
            Ok(final_result)
        };

        let result = if use_bg_thread_pool {
            self.thread_pool_clean.install(scan_and_hash)
        } else {
            scan_and_hash()
        };
        self.assert_safe_squashing_accounts_hash(
            storages.max_slot_inclusive(),
            config.epoch_schedule,
        );
        stats.log();
        result
    }

    /// return alive roots to retain, even though they are ancient
    fn calc_alive_ancient_historical_roots(&self, min_root: Slot) -> HashSet<Slot> {
        let mut ancient_alive_roots = HashSet::default();
        {
            let all_roots = self.accounts_index.roots_tracker.read().unwrap();

            if let Some(min) = all_roots.historical_roots.min() {
                for slot in min..min_root {
                    if all_roots.alive_roots.contains(&slot) {
                        // there was a storage for this root, so it counts as a root
                        ancient_alive_roots.insert(slot);
                    }
                }
            }
        }
        ancient_alive_roots
    }

    /// get rid of historical roots that are older than 'min_root'.
    /// These will be older than an epoch from a current root.
    fn remove_old_historical_roots(&self, min_root: Slot) {
        let alive_roots = self.calc_alive_ancient_historical_roots(min_root);
        self.accounts_index
            .remove_old_historical_roots(min_root, &alive_roots);
    }

    /// Only called from startup or test code.
    pub fn verify_bank_hash_and_lamports(
        &self,
        slot: Slot,
        ancestors: &Ancestors,
        total_lamports: u64,
        test_hash_calculation: bool,
        epoch_schedule: &EpochSchedule,
        rent_collector: &RentCollector,
        can_cached_slot_be_unflushed: bool,
        enable_rehashing: bool,
    ) -> Result<(), BankHashVerificationError> {
        self.verify_bank_hash_and_lamports_new(
            slot,
            ancestors,
            total_lamports,
            test_hash_calculation,
            epoch_schedule,
            rent_collector,
            can_cached_slot_be_unflushed,
            false,
            false,
            enable_rehashing,
        )
    }

    /// Only called from startup or test code.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_bank_hash_and_lamports_new(
        &self,
        slot: Slot,
        ancestors: &Ancestors,
        total_lamports: u64,
        test_hash_calculation: bool,
        epoch_schedule: &EpochSchedule,
        rent_collector: &RentCollector,
        can_cached_slot_be_unflushed: bool,
        ignore_mismatch: bool,
        store_hash_raw_data_for_debug: bool,
        enable_rehashing: bool,
    ) -> Result<(), BankHashVerificationError> {
        use BankHashVerificationError::*;

        let use_index = false;
        let check_hash = false; // this will not be supported anymore
                                // interesting to consider this
        let is_startup = true;
        let (calculated_hash, calculated_lamports) = self
            .calculate_accounts_hash_helper_with_verify(
                use_index,
                test_hash_calculation,
                slot,
                CalcAccountsHashConfig {
                    use_bg_thread_pool: !is_startup,
                    check_hash,
                    ancestors: Some(ancestors),
                    use_write_cache: can_cached_slot_be_unflushed,
                    epoch_schedule,
                    rent_collector,
                    store_detailed_debug_info_on_failure: store_hash_raw_data_for_debug,
                    full_snapshot: None,
                    enable_rehashing,
                },
                None,
            )?;

        if calculated_lamports != total_lamports {
            warn!(
                "Mismatched total lamports: {} calculated: {}",
                total_lamports, calculated_lamports
            );
            return Err(MismatchedTotalLamports(calculated_lamports, total_lamports));
        }

        if ignore_mismatch {
            Ok(())
        } else {
            let bank_hashes = self.bank_hashes.read().unwrap();
            if let Some(found_hash_info) = bank_hashes.get(&slot) {
                if calculated_hash == found_hash_info.snapshot_hash {
                    Ok(())
                } else {
                    warn!(
                        "mismatched bank hash for slot {}: {} (calculated) != {} (expected)",
                        slot, calculated_hash, found_hash_info.snapshot_hash
                    );
                    Err(MismatchedBankHash)
                }
            } else {
                Err(MissingBankHash)
            }
        }
    }

    /// Perform the scan for pubkeys that were written to in a slot
    fn do_scan_slot_for_dirty_pubkeys(
        &self,
        slot: Slot,
    ) -> ScanStorageResult<Pubkey, DashSet<Pubkey>> {
        self.scan_account_storage(
            slot,
            |loaded_account: LoadedAccount| Some(*loaded_account.pubkey()),
            |accum: &DashSet<Pubkey>, loaded_account: LoadedAccount| {
                accum.insert(*loaded_account.pubkey());
            },
        )
    }

    /// Reduce the scan result of dirty pubkeys after calling `scan_account_storage()` into a
    /// single vec of Pubkeys.
    fn do_reduce_scan_slot_for_dirty_pubkeys(
        scan_result: ScanStorageResult<Pubkey, DashSet<Pubkey>>,
    ) -> Vec<Pubkey> {
        match scan_result {
            ScanStorageResult::Cached(cached_result) => cached_result,
            ScanStorageResult::Stored(stored_result) => {
                stored_result.into_iter().collect::<Vec<_>>()
            }
        }
    }

    /// Scan a slot for dirty pubkeys
    fn scan_slot_for_dirty_pubkeys(&self, slot: Slot) -> Vec<Pubkey> {
        let dirty_pubkeys = self.do_scan_slot_for_dirty_pubkeys(slot);
        Self::do_reduce_scan_slot_for_dirty_pubkeys(dirty_pubkeys)
    }

    /// Scan a slot in the account storage for dirty pubkeys and insert them into the list of
    /// uncleaned pubkeys
    ///
    /// This function is called in Bank::drop() when the bank is _not_ frozen, so that its pubkeys
    /// are considered for cleanup.
    pub fn scan_slot_and_insert_dirty_pubkeys_into_uncleaned_pubkeys(&self, slot: Slot) {
        let dirty_pubkeys = self.scan_slot_for_dirty_pubkeys(slot);
        self.uncleaned_pubkeys.insert(slot, dirty_pubkeys);
    }

    pub fn get_accounts_delta_hash(&self, slot: Slot) -> Hash {
        self.get_accounts_delta_hash_with_rewrites(slot, &Rewrites::default())
    }

    /// helper to return
    /// 1. pubkey, hash pairs for the slot
    /// 2. us spent scanning
    /// 3. Measure started when we began accumulating
    fn get_pubkey_hash_for_slot(&self, slot: Slot) -> (Vec<(Pubkey, Hash)>, u64, Measure) {
        let mut scan = Measure::start("scan");

        let scan_result: ScanStorageResult<(Pubkey, Hash), DashMapVersionHash> = self
            .scan_account_storage(
                slot,
                |loaded_account: LoadedAccount| {
                    // Cache only has one version per key, don't need to worry about versioning
                    Some((*loaded_account.pubkey(), loaded_account.loaded_hash()))
                },
                |accum: &DashMap<Pubkey, (u64, Hash)>, loaded_account: LoadedAccount| {
                    let loaded_write_version = loaded_account.write_version();
                    let loaded_hash = loaded_account.loaded_hash();
                    // keep the latest write version for each pubkey
                    match accum.entry(*loaded_account.pubkey()) {
                        Occupied(mut occupied_entry) => {
                            if loaded_write_version > occupied_entry.get().version() {
                                occupied_entry.insert((loaded_write_version, loaded_hash));
                            }
                        }

                        Vacant(vacant_entry) => {
                            vacant_entry.insert((loaded_write_version, loaded_hash));
                        }
                    }
                },
            );
        scan.stop();

        let accumulate = Measure::start("accumulate");
        let hashes: Vec<_> = match scan_result {
            ScanStorageResult::Cached(cached_result) => cached_result,
            ScanStorageResult::Stored(stored_result) => stored_result
                .into_iter()
                .map(|(pubkey, (_latest_write_version, hash))| (pubkey, hash))
                .collect(),
        };
        (hashes, scan.as_us(), accumulate)
    }

    pub fn get_accounts_delta_hash_with_rewrites(
        &self,
        slot: Slot,
        skipped_rewrites: &Rewrites,
    ) -> Hash {
        let (mut hashes, scan_us, mut accumulate) = self.get_pubkey_hash_for_slot(slot);
        let dirty_keys = hashes.iter().map(|(pubkey, _hash)| *pubkey).collect();

        if self.filler_accounts_enabled() {
            // filler accounts must be added to 'dirty_keys' above but cannot be used to calculate hash
            hashes.retain(|(pubkey, _hash)| !self.is_filler_account(pubkey));
        }

        self.extend_hashes_with_skipped_rewrites(&mut hashes, skipped_rewrites);

        let ret = AccountsHash::accumulate_account_hashes(hashes);
        accumulate.stop();
        let mut uncleaned_time = Measure::start("uncleaned_index");
        self.uncleaned_pubkeys.insert(slot, dirty_keys);
        uncleaned_time.stop();
        self.stats
            .store_uncleaned_update
            .fetch_add(uncleaned_time.as_us(), Ordering::Relaxed);

        self.stats
            .delta_hash_scan_time_total_us
            .fetch_add(scan_us, Ordering::Relaxed);
        self.stats
            .delta_hash_accumulate_time_total_us
            .fetch_add(accumulate.as_us(), Ordering::Relaxed);
        self.stats.delta_hash_num.fetch_add(1, Ordering::Relaxed);
        ret
    }

    /// add all items from 'skipped_rewrites' to 'hashes' where the pubkey doesn't already exist in 'hashes'
    fn extend_hashes_with_skipped_rewrites(
        &self,
        hashes: &mut Vec<(Pubkey, Hash)>,
        skipped_rewrites: &Rewrites,
    ) {
        let mut skipped_rewrites = skipped_rewrites.read().unwrap().clone();
        hashes.iter().for_each(|(key, _)| {
            skipped_rewrites.remove(key);
        });

        if self.filler_accounts_enabled() {
            // simulate the time we would normally spend hashing the filler accounts
            // this is an over approximation but at least takes a stab at simulating what the validator would spend time doing
            let _ = AccountsHash::accumulate_account_hashes(
                skipped_rewrites
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .collect::<Vec<_>>(),
            );

            // filler accounts do not get their updated hash values hashed into the delta hash
            skipped_rewrites.retain(|key, _| !self.is_filler_account(key));
        }

        hashes.extend(skipped_rewrites.into_iter());
    }

    fn update_index<'a, T: ReadableAccount + Sync>(
        &self,
        infos: Vec<AccountInfo>,
        accounts: impl StorableAccounts<'a, T>,
        reclaim: UpsertReclaim,
    ) -> SlotList<AccountInfo> {
        let target_slot = accounts.target_slot();
        // using a thread pool here results in deadlock panics from bank_hashes.write()
        // so, instead we limit how many threads will be created to the same size as the bg thread pool
        let len = std::cmp::min(accounts.len(), infos.len());
        let threshold = 1;
        let update = |start, end| {
            let mut reclaims = Vec::with_capacity((end - start) / 2);

            (start..end).into_iter().for_each(|i| {
                let info = infos[i];
                let pubkey_account = (accounts.pubkey(i), accounts.account(i));
                let pubkey = pubkey_account.0;
                let old_slot = accounts.slot(i);
                self.accounts_index.upsert(
                    target_slot,
                    old_slot,
                    pubkey,
                    pubkey_account.1,
                    &self.account_indexes,
                    info,
                    &mut reclaims,
                    reclaim,
                );
            });
            reclaims
        };
        if len > threshold {
            let chunk_size = std::cmp::max(1, len / quarter_thread_count()); // # pubkeys/thread
            let batches = 1 + len / chunk_size;
            (0..batches)
                .into_par_iter()
                .map(|batch| {
                    let start = batch * chunk_size;
                    let end = std::cmp::min(start + chunk_size, len);
                    update(start, end)
                })
                .flatten()
                .collect::<Vec<_>>()
        } else {
            update(0, len)
        }
    }
