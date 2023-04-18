    fn store_accounts_custom<'a, 'b, T: ReadableAccount + Sync + ZeroLamport>(
        &'a self,
        accounts: impl StorableAccounts<'b, T>,
        hashes: Option<&[impl Borrow<Hash>]>,
        storage_finder: Option<StorageFinder<'a>>,
        write_version_producer: Option<Box<dyn Iterator<Item = u64>>>,
        is_cached_store: bool,
        reset_accounts: bool,
    ) -> StoreAccountsTiming {
        let slot = accounts.target_slot();
        let storage_finder = storage_finder
            .unwrap_or_else(|| Box::new(move |slot, size| self.find_storage_candidate(slot, size)));

        let write_version_producer: Box<dyn Iterator<Item = u64>> = write_version_producer
            .unwrap_or_else(|| {
                let mut current_version = self.bulk_assign_write_version(accounts.len());
                Box::new(std::iter::from_fn(move || {
                    let ret = current_version;
                    current_version += 1;
                    Some(ret)
                }))
            });

        self.stats
            .store_num_accounts
            .fetch_add(accounts.len() as u64, Ordering::Relaxed);
        let mut store_accounts_time = Measure::start("store_accounts");
        let infos = self.store_accounts_to(
            &accounts,
            hashes,
            storage_finder,
            write_version_producer,
            is_cached_store,
        );
        store_accounts_time.stop();
        self.stats
            .store_accounts
            .fetch_add(store_accounts_time.as_us(), Ordering::Relaxed);
        let mut update_index_time = Measure::start("update_index");

        let previous_slot_entry_was_cached = self.caching_enabled && is_cached_store;

        // If the cache was flushed, then because `update_index` occurs
        // after the account are stored by the above `store_accounts_to`
        // call and all the accounts are stored, all reads after this point
        // will know to not check the cache anymore
        let mut reclaims = self.update_index(infos, accounts, previous_slot_entry_was_cached);

        // For each updated account, `reclaims` should only have at most one
        // item (if the account was previously updated in this slot).
        // filter out the cached reclaims as those don't actually map
        // to anything that needs to be cleaned in the backing storage
        // entries
        if self.caching_enabled {
            reclaims.retain(|(_, r)| !r.is_cached());

            if is_cached_store {
                assert!(reclaims.is_empty());
            }
        }

        update_index_time.stop();
        self.stats
            .store_update_index
            .fetch_add(update_index_time.as_us(), Ordering::Relaxed);

        // A store for a single slot should:
        // 1) Only make "reclaims" for the same slot
        // 2) Should not cause any slots to be removed from the storage
        // database because
        //    a) this slot  has at least one account (the one being stored),
        //    b)From 1) we know no other slots are included in the "reclaims"
        //
        // From 1) and 2) we guarantee passing `no_purge_stats` == None, which is
        // equivalent to asserting there will be no dead slots, is safe.
        let no_purge_stats = None;
        let mut handle_reclaims_time = Measure::start("handle_reclaims");
        self.handle_reclaims(&reclaims, Some(slot), no_purge_stats, None, reset_accounts);
        handle_reclaims_time.stop();
        self.stats
            .store_handle_reclaims
            .fetch_add(handle_reclaims_time.as_us(), Ordering::Relaxed);

        StoreAccountsTiming {
            store_accounts_elapsed: store_accounts_time.as_us(),
            update_index_elapsed: update_index_time.as_us(),
            handle_reclaims_elapsed: handle_reclaims_time.as_us(),
        }
    }

    pub fn add_root(&self, slot: Slot) -> AccountsAddRootTiming {
        let mut index_time = Measure::start("index_add_root");
        self.accounts_index.add_root(slot, self.caching_enabled);
        index_time.stop();
        let mut cache_time = Measure::start("cache_add_root");
        if self.caching_enabled {
            self.accounts_cache.add_root(slot);
        }
        cache_time.stop();
        let mut store_time = Measure::start("store_add_root");
        if let Some(slot_stores) = self.storage.get_slot_stores(slot) {
            for (store_id, store) in slot_stores.read().unwrap().iter() {
                self.dirty_stores.insert((slot, *store_id), store.clone());
            }
        }
        store_time.stop();

        AccountsAddRootTiming {
            index_us: index_time.as_us(),
            cache_us: cache_time.as_us(),
            store_us: store_time.as_us(),
        }
    }

    pub fn get_snapshot_storages(
        &self,
        snapshot_slot: Slot,
        snapshot_base_slot: Option<Slot>,
        ancestors: Option<&Ancestors>,
    ) -> (SnapshotStorages, Vec<Slot>) {
        let mut m = Measure::start("get slots");
        let slots = self
            .storage
            .map
            .iter()
            .map(|k| *k.key() as Slot)
            .collect::<Vec<_>>();
        m.stop();
        let mut m2 = Measure::start("filter");

        let chunk_size = 5_000;
        let wide = self.thread_pool_clean.install(|| {
            slots
                .par_chunks(chunk_size)
                .map(|slots| {
                    slots
                        .iter()
                        .filter_map(|slot| {
                            if *slot <= snapshot_slot
                                && snapshot_base_slot
                                    .map_or(true, |snapshot_base_slot| *slot > snapshot_base_slot)
                                && (self.accounts_index.is_root(*slot)
                                    || ancestors
                                        .map(|ancestors| ancestors.contains_key(slot))
                                        .unwrap_or_default())
                            {
                                self.storage.map.get(slot).map_or_else(
                                    || None,
                                    |item| {
                                        let storages = item
                                            .value()
                                            .read()
                                            .unwrap()
                                            .values()
                                            .filter(|x| x.has_accounts())
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        if !storages.is_empty() {
                                            Some((storages, *slot))
                                        } else {
                                            None
                                        }
                                    },
                                )
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<(SnapshotStorage, Slot)>>()
                })
                .collect::<Vec<_>>()
        });
        m2.stop();
        let mut m3 = Measure::start("flatten");
        // some slots we found above may not have been a root or met the slot # constraint.
        // So the resulting 'slots' vector we return will be a subset of the raw keys we got initially.
        let mut slots = Vec::with_capacity(slots.len());
        let result = wide
            .into_iter()
            .flatten()
            .map(|(storage, slot)| {
                slots.push(slot);
                storage
            })
            .collect::<Vec<_>>();
        m3.stop();

        debug!(
            "hash_total: get slots: {}, filter: {}, flatten: {}",
            m.as_us(),
            m2.as_us(),
            m3.as_us()
        );
        (result, slots)
    }

    fn process_storage_slot<'a>(
        &self,
        storage_maps: &'a [Arc<AccountStorageEntry>],
    ) -> GenerateIndexAccountsMap<'a> {
        let num_accounts = storage_maps
            .iter()
            .map(|storage| storage.approx_stored_count())
            .sum();
        let mut accounts_map = GenerateIndexAccountsMap::with_capacity(num_accounts);
        storage_maps.iter().for_each(|storage| {
            let accounts = storage.all_accounts();
            accounts.into_iter().for_each(|stored_account| {
                let this_version = stored_account.meta.write_version;
                let pubkey = stored_account.meta.pubkey;
                assert!(!self.is_filler_account(&pubkey));
                match accounts_map.entry(pubkey) {
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(IndexAccountMapEntry {
                            write_version: this_version,
                            store_id: storage.append_vec_id(),
                            stored_account,
                        });
                    }
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        let occupied_version = entry.get().write_version;
                        if occupied_version < this_version {
                            entry.insert(IndexAccountMapEntry {
                                write_version: this_version,
                                store_id: storage.append_vec_id(),
                                stored_account,
                            });
                        } else {
                            assert_ne!(occupied_version, this_version);
                        }
                    }
                }
            })
        });
        accounts_map
    }

    fn generate_index_for_slot<'a>(
        &self,
        accounts_map: GenerateIndexAccountsMap<'a>,
        slot: &Slot,
        rent_collector: &RentCollector,
    ) -> SlotIndexGenerationInfo {
        if accounts_map.is_empty() {
            return SlotIndexGenerationInfo::default();
        }

        let secondary = !self.account_indexes.is_empty();

        let mut accounts_data_len = 0;
        let mut num_accounts_rent_exempt = 0;
        let num_accounts = accounts_map.len();
        let items = accounts_map.into_iter().map(
            |(
                pubkey,
                IndexAccountMapEntry {
                    write_version: _write_version,
                    store_id,
                    stored_account,
                },
            )| {
                if secondary {
                    self.accounts_index.update_secondary_indexes(
                        &pubkey,
                        &stored_account,
                        &self.account_indexes,
                    );
                }
                if !stored_account.is_zero_lamport() {
                    accounts_data_len += stored_account.data().len() as u64;
                }

                if !rent_collector.should_collect_rent(&pubkey, &stored_account)
                    || rent_collector.get_rent_due(&stored_account).is_exempt()
                {
                    num_accounts_rent_exempt += 1;
                }

                (
                    pubkey,
                    AccountInfo::new(
                        StorageLocation::AppendVec(store_id, stored_account.offset), // will never be cached
                        stored_account.stored_size as StoredSize, // stored_size should never exceed StoredSize::MAX because of max data len const
                        stored_account.account_meta.lamports,
                    ),
                )
            },
        );

        let (dirty_pubkeys, insert_time_us) = self
            .accounts_index
            .insert_new_if_missing_into_primary_index(*slot, num_accounts, items);

        // dirty_pubkeys will contain a pubkey if an item has multiple rooted entries for
        // a given pubkey. If there is just a single item, there is no cleaning to
        // be done on that pubkey. Use only those pubkeys with multiple updates.
        if !dirty_pubkeys.is_empty() {
            self.uncleaned_pubkeys.insert(*slot, dirty_pubkeys);
        }
        SlotIndexGenerationInfo {
            insert_time_us,
            num_accounts: num_accounts as u64,
            num_accounts_rent_exempt,
            accounts_data_len,
        }
    }

    fn filler_unique_id_bytes() -> usize {
        std::mem::size_of::<u32>()
    }

    fn filler_rent_partition_prefix_bytes() -> usize {
        std::mem::size_of::<u64>()
    }

    fn filler_prefix_bytes() -> usize {
        Self::filler_unique_id_bytes() + Self::filler_rent_partition_prefix_bytes()
    }

    pub fn is_filler_account_helper(
        pubkey: &Pubkey,
        filler_account_suffix: Option<&Pubkey>,
    ) -> bool {
        let offset = Self::filler_prefix_bytes();
        filler_account_suffix
            .as_ref()
            .map(|filler_account_suffix| {
                pubkey.as_ref()[offset..] == filler_account_suffix.as_ref()[offset..]
            })
            .unwrap_or_default()
    }

    /// true if 'pubkey' is a filler account
    pub fn is_filler_account(&self, pubkey: &Pubkey) -> bool {
        Self::is_filler_account_helper(pubkey, self.filler_account_suffix.as_ref())
    }

    /// true if it is possible that there are filler accounts present
    pub fn filler_accounts_enabled(&self) -> bool {
        self.filler_account_suffix.is_some()
    }

    /// retain slots in 'roots' that are > (max(roots) - slots_per_epoch)
    fn retain_roots_within_one_epoch_range(roots: &mut Vec<Slot>, slots_per_epoch: SlotCount) {
        if let Some(max) = roots.iter().max() {
            let min = max - slots_per_epoch;
            roots.retain(|slot| slot > &min);
        }
    }

    /// filler accounts are space-holding accounts which are ignored by hash calculations and rent.
    /// They are designed to allow a validator to run against a network successfully while simulating having many more accounts present.
    /// All filler accounts share a common pubkey suffix. The suffix is randomly generated per validator on startup.
    /// The filler accounts are added to each slot in the snapshot after index generation.
    /// The accounts added in a slot are setup to have pubkeys such that rent will be collected from them before (or when?) their slot becomes an epoch old.
    /// Thus, the filler accounts are rewritten by rent and the old slot can be thrown away successfully.
    pub fn maybe_add_filler_accounts(&self, epoch_schedule: &EpochSchedule) {
        if self.filler_account_count == 0 {
            return;
        }

        let max_root_inclusive = self.accounts_index.max_root_inclusive();
        let epoch = epoch_schedule.get_epoch(max_root_inclusive);

        info!("adding {} filler accounts", self.filler_account_count);
        // break this up to force the accounts out of memory after each pass
        let passes = 100;
        let mut roots = self.storage.all_slots();
        Self::retain_roots_within_one_epoch_range(
            &mut roots,
            epoch_schedule.get_slots_in_epoch(epoch),
        );
        let root_count = roots.len();
        let per_pass = std::cmp::max(1, root_count / passes);
        let overall_index = AtomicUsize::new(0);
        let string = "FiLLERACCoUNTooooooooooooooooooooooooooooooo";
        let hash = Hash::from_str(string).unwrap();
        let owner = Pubkey::from_str(string).unwrap();
        let lamports = 100_000_000;
        let space = 0;
        let account = AccountSharedData::new(lamports, space, &owner);
        let added = AtomicUsize::default();
        for pass in 0..=passes {
            self.accounts_index.set_startup(true);
            let roots_in_this_pass = roots
                .iter()
                .skip(pass * per_pass)
                .take(per_pass)
                .collect::<Vec<_>>();
            roots_in_this_pass.into_par_iter().for_each(|slot| {
                let storage_maps: Vec<Arc<AccountStorageEntry>> = self
                    .storage
                    .get_slot_storage_entries(*slot)
                    .unwrap_or_default();
                if storage_maps.is_empty() {
                    return;
                }

                let partition = crate::bank::Bank::variable_cycle_partition_from_previous_slot(
                    epoch_schedule,
                    *slot,
                );
                let subrange = crate::bank::Bank::pubkey_range_from_partition(partition);

                let idx = overall_index.fetch_add(1, Ordering::Relaxed);
                let filler_entries = (idx + 1) * self.filler_account_count / root_count
                    - idx * self.filler_account_count / root_count;
                let accounts = (0..filler_entries)
                    .map(|_| {
                        let my_id = added.fetch_add(1, Ordering::Relaxed);
                        let my_id_bytes = u32::to_be_bytes(my_id as u32);

                        // pubkey begins life as entire filler 'suffix' pubkey
                        let mut key = self.filler_account_suffix.unwrap();
                        let rent_prefix_bytes = Self::filler_rent_partition_prefix_bytes();
                        // first bytes are replaced with rent partition range: filler_rent_partition_prefix_bytes
                        key.as_mut()[0..rent_prefix_bytes]
                            .copy_from_slice(&subrange.start().as_ref()[0..rent_prefix_bytes]);
                        // next bytes are replaced with my_id: filler_unique_id_bytes
                        key.as_mut()[rent_prefix_bytes
                            ..(rent_prefix_bytes + Self::filler_unique_id_bytes())]
                            .copy_from_slice(&my_id_bytes);
                        assert!(subrange.contains(&key));
                        key
                    })
                    .collect::<Vec<_>>();
                let add = accounts
                    .iter()
                    .map(|key| (key, &account))
                    .collect::<Vec<_>>();
                let hashes = (0..filler_entries).map(|_| hash).collect::<Vec<_>>();
                self.store_accounts_frozen((*slot, &add[..]), Some(&hashes[..]), None, None);
            });
            self.accounts_index.set_startup(false);
        }
        info!("added {} filler accounts", added.load(Ordering::Relaxed));
    }

    #[allow(clippy::needless_collect)]
    pub fn generate_index(
        &self,
        limit_load_slot_count_from_snapshot: Option<usize>,
        verify: bool,
        genesis_config: &GenesisConfig,
    ) -> IndexGenerationInfo {
        let mut slots = self.storage.all_slots();
        #[allow(clippy::stable_sort_primitive)]
        slots.sort();
        if let Some(limit) = limit_load_slot_count_from_snapshot {
            slots.truncate(limit); // get rid of the newer slots and keep just the older
        }
        let max_slot = slots.last().cloned().unwrap_or_default();
        let schedule = genesis_config.epoch_schedule;
        let rent_collector = RentCollector::new(
            schedule.get_epoch(max_slot),
            &schedule,
            genesis_config.slots_per_year(),
            &genesis_config.rent,
        );
        let accounts_data_len = AtomicU64::new(0);

        // pass == 0 always runs and generates the index
        // pass == 1 only runs if verify == true.
        // verify checks that all the expected items are in the accounts index and measures how long it takes to look them all up
        let passes = if verify { 2 } else { 1 };
        for pass in 0..passes {
            if pass == 0 {
                self.accounts_index.set_startup(true);
            }
            let storage_info = StorageSizeAndCountMap::default();
            let total_processed_slots_across_all_threads = AtomicU64::new(0);
            let outer_slots_len = slots.len();
            let threads = if self.accounts_index.is_disk_index_enabled() {
                // these write directly to disk, so the more threads, the better
                num_cpus::get()
            } else {
                // seems to be a good hueristic given varying # cpus for in-mem disk index
                8
            };
            let chunk_size = (outer_slots_len / (std::cmp::max(1, threads.saturating_sub(1)))) + 1; // approximately 400k slots in a snapshot
            let mut index_time = Measure::start("index");
            let insertion_time_us = AtomicU64::new(0);
            let rent_exempt = AtomicU64::new(0);
            let total_duplicates = AtomicU64::new(0);
            let storage_info_timings = Mutex::new(GenerateIndexTimings::default());
            let scan_time: u64 = slots
                .par_chunks(chunk_size)
                .map(|slots| {
                    let mut log_status = MultiThreadProgress::new(
                        &total_processed_slots_across_all_threads,
                        2,
                        outer_slots_len as u64,
                    );
                    let mut scan_time_sum = 0;
                    for (index, slot) in slots.iter().enumerate() {
                        let mut scan_time = Measure::start("scan");
                        log_status.report(index as u64);
                        let storage_maps: Vec<Arc<AccountStorageEntry>> = self
                            .storage
                            .get_slot_storage_entries(*slot)
                            .unwrap_or_default();
                        let accounts_map = self.process_storage_slot(&storage_maps);
                        scan_time.stop();
                        scan_time_sum += scan_time.as_us();
                        Self::update_storage_info(
                            &storage_info,
                            &accounts_map,
                            &storage_info_timings,
                        );

                        let insert_us = if pass == 0 {
                            // generate index
                            let SlotIndexGenerationInfo {
                                insert_time_us: insert_us,
                                num_accounts: total_this_slot,
                                num_accounts_rent_exempt: rent_exempt_this_slot,
                                accounts_data_len: accounts_data_len_this_slot,
                            } = self.generate_index_for_slot(accounts_map, slot, &rent_collector);
                            rent_exempt.fetch_add(rent_exempt_this_slot, Ordering::Relaxed);
                            total_duplicates.fetch_add(total_this_slot, Ordering::Relaxed);
                            accounts_data_len
                                .fetch_add(accounts_data_len_this_slot, Ordering::Relaxed);
                            insert_us
                        } else {
                            // verify index matches expected and measure the time to get all items
                            assert!(verify);
                            let mut lookup_time = Measure::start("lookup_time");
                            for account in accounts_map.into_iter() {
                                let (key, account_info) = account;
                                let lock = self.accounts_index.get_account_maps_read_lock(&key);
                                let x = lock.get(&key).unwrap();
                                let sl = x.slot_list.read().unwrap();
                                let mut count = 0;
                                for (slot2, account_info2) in sl.iter() {
                                    if slot2 == slot {
                                        count += 1;
                                        let ai = AccountInfo::new(
                                            StorageLocation::AppendVec(
                                                account_info.store_id,
                                                account_info.stored_account.offset,
                                            ), // will never be cached
                                            account_info.stored_account.stored_size as StoredSize, // stored_size should never exceed StoredSize::MAX because of max data len const
                                            account_info.stored_account.account_meta.lamports,
                                        );
                                        assert_eq!(&ai, account_info2);
                                    }
                                }
                                assert_eq!(1, count);
                            }
                            lookup_time.stop();
                            lookup_time.as_us()
                        };
                        insertion_time_us.fetch_add(insert_us, Ordering::Relaxed);
                    }
                    scan_time_sum
                })
                .sum();
            index_time.stop();

            info!("rent_collector: {:?}", rent_collector);
            let mut min_bin_size = usize::MAX;
            let mut max_bin_size = usize::MIN;
            let total_items = self
                .accounts_index
                .account_maps
                .iter()
                .map(|map_bin| {
                    let len = map_bin.read().unwrap().len_for_stats();
                    min_bin_size = std::cmp::min(min_bin_size, len);
                    max_bin_size = std::cmp::max(max_bin_size, len);
                    len as usize
                })
                .sum();

            // subtract data.len() from accounts_data_len for all old accounts that are in the index twice
            let mut accounts_data_len_dedup_timer =
                Measure::start("handle accounts data len duplicates");
            if pass == 0 {
                let mut unique_pubkeys = HashSet::<Pubkey>::default();
                self.uncleaned_pubkeys.iter().for_each(|entry| {
                    entry.value().iter().for_each(|pubkey| {
                        unique_pubkeys.insert(*pubkey);
                    })
                });
                let accounts_data_len_from_duplicates = unique_pubkeys
                    .into_iter()
                    .collect::<Vec<_>>()
                    .par_chunks(4096)
                    .map(|pubkeys| self.pubkeys_to_duplicate_accounts_data_len(pubkeys))
                    .sum();
                accounts_data_len.fetch_sub(accounts_data_len_from_duplicates, Ordering::Relaxed);
                info!(
                    "accounts data len: {}",
                    accounts_data_len.load(Ordering::Relaxed)
                );
            }
            accounts_data_len_dedup_timer.stop();

            let storage_info_timings = storage_info_timings.into_inner().unwrap();

            let mut index_flush_us = 0;
            if pass == 0 {
                // tell accounts index we are done adding the initial accounts at startup
                let mut m = Measure::start("accounts_index_idle_us");
                self.accounts_index.set_startup(false);
                m.stop();
                index_flush_us = m.as_us();
            }

            let mut timings = GenerateIndexTimings {
                index_flush_us,
                scan_time,
                index_time: index_time.as_us(),
                insertion_time_us: insertion_time_us.load(Ordering::Relaxed),
                min_bin_size,
                max_bin_size,
                total_items,
                rent_exempt: rent_exempt.load(Ordering::Relaxed),
                total_duplicates: total_duplicates.load(Ordering::Relaxed),
                storage_size_accounts_map_us: storage_info_timings.storage_size_accounts_map_us,
                storage_size_accounts_map_flatten_us: storage_info_timings
                    .storage_size_accounts_map_flatten_us,
                accounts_data_len_dedup_time_us: accounts_data_len_dedup_timer.as_us(),
                ..GenerateIndexTimings::default()
            };

            if pass == 0 {
                // Need to add these last, otherwise older updates will be cleaned
                for slot in &slots {
                    self.accounts_index.add_root(*slot, false);
                }

                self.set_storage_count_and_alive_bytes(storage_info, &mut timings);
            }
            timings.report();
        }

        IndexGenerationInfo {
            accounts_data_len: accounts_data_len.load(Ordering::Relaxed),
        }
    }

    /// Used during generate_index() to get the _duplicate_ accounts data len from the given pubkeys
    /// Note this should only be used when ALL entries in the accounts index are roots.
    fn pubkeys_to_duplicate_accounts_data_len(&self, pubkeys: &[Pubkey]) -> u64 {
        let mut accounts_data_len_from_duplicates = 0;
        pubkeys.iter().for_each(|pubkey| {
            if let Some(entry) = self.accounts_index.get_account_read_entry(pubkey) {
                let slot_list = entry.slot_list();
                if slot_list.len() < 2 {
                    return;
                }
                // Only the account data len in the highest slot should be used, and the rest are
                // duplicates.  So sort the slot list in descending slot order, skip the first
                // item, then sum up the remaining data len, which are the duplicates.
                let mut slot_list = slot_list.clone();
                slot_list
                    .select_nth_unstable_by(0, |a, b| b.0.cmp(&a.0))
                    .2
                    .iter()
                    .for_each(|(slot, account_info)| {
                        let maybe_storage_entry = self
                            .storage
                            .get_account_storage_entry(*slot, account_info.store_id());
                        let mut accessor = LoadedAccountAccessor::Stored(
                            maybe_storage_entry.map(|entry| (entry, account_info.offset())),
                        );
                        let loaded_account = accessor.check_and_get_loaded_account();
                        accounts_data_len_from_duplicates += loaded_account.data().len();
                    });
            }
        });
        accounts_data_len_from_duplicates as u64
    }

    fn update_storage_info(
        storage_info: &StorageSizeAndCountMap,
        accounts_map: &GenerateIndexAccountsMap<'_>,
        timings: &Mutex<GenerateIndexTimings>,
    ) {
        let mut storage_size_accounts_map_time = Measure::start("storage_size_accounts_map");

        let mut storage_info_local = HashMap::<AppendVecId, StorageSizeAndCount>::default();
        // first collect into a local HashMap with no lock contention
        for (_, v) in accounts_map.iter() {
            let mut info = storage_info_local
                .entry(v.store_id)
                .or_insert_with(StorageSizeAndCount::default);
            info.stored_size += v.stored_account.stored_size;
            info.count += 1;
        }
        storage_size_accounts_map_time.stop();
        // second, collect into the shared DashMap once we've figured out all the info per store_id
        let mut storage_size_accounts_map_flatten_time =
            Measure::start("storage_size_accounts_map_flatten_time");
        for (store_id, v) in storage_info_local.into_iter() {
            let mut info = storage_info
                .entry(store_id)
                .or_insert_with(StorageSizeAndCount::default);
            info.stored_size += v.stored_size;
            info.count += v.count;
        }
        storage_size_accounts_map_flatten_time.stop();

        let mut timings = timings.lock().unwrap();
        timings.storage_size_accounts_map_us += storage_size_accounts_map_time.as_us();
        timings.storage_size_accounts_map_flatten_us +=
            storage_size_accounts_map_flatten_time.as_us();
    }
