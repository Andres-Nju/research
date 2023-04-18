    fn unref_accounts_already_in_storage(
        &self,
        accounts: &[(&Pubkey, &StoredAccountMeta<'_>, u64)],
        existing_ancient_pubkeys: &mut HashSet<Pubkey>,
    ) {
        let mut unref = HashSet::<&Pubkey>::default();
        // for each key that we're about to add that already exists in this storage, we need to unref. The account was in a different storage.
        // Now it is being put into an ancient storage again, but it is already there, so maintain max of 1 ref per storage in the accounts index.
        // The slot that currently references the account is going away, so unref to maintain # slots that reference the pubkey = refcount.
        accounts.iter().for_each(|(key, _, _)| {
            if !existing_ancient_pubkeys.insert(**key) {
                // this key exists BOTH in 'accounts' and already in the ancient append vec, so we need to unref it
                unref.insert(*key);
            }
        });

        self.thread_pool_clean.install(|| {
            unref.into_par_iter().for_each(|key| {
                self.accounts_index.unref_from_storage(key);
            });
        });
    }

    /// helper function to cleanup call to 'store_accounts_frozen'
    fn store_ancient_accounts(
        &self,
        ancient_slot: Slot,
        ancient_store: &Arc<AccountStorageEntry>,
        accounts: &AccountsToStore,
        storage_selector: StorageSelector,
    ) -> StoreAccountsTiming {
        let (accounts, hashes) = accounts.get(storage_selector);

        self.store_accounts_frozen(
            (ancient_slot, accounts),
            Some(hashes),
            Some(ancient_store),
            None,
            StoreReclaims::Ignore,
        )
    }

    /// get the storages from 'slot' to squash
    /// or None if this slot should be skipped
    fn get_storages_to_move_to_ancient_append_vec(
        &self,
        slot: Slot,
        current_ancient: &mut Option<(Slot, Arc<AccountStorageEntry>)>,
    ) -> Option<SnapshotStorage> {
        self.get_storages_for_slot(slot).and_then(|all_storages| {
            self.should_move_to_ancient_append_vec(&all_storages, current_ancient, slot)
                .then_some(all_storages)
        })
    }

    /// return true if the accounts in this slot should be moved to an ancient append vec
    /// otherwise, return false and the caller can skip this slot
    /// side effect could be updating 'current_ancient'
    pub fn should_move_to_ancient_append_vec(
        &self,
        all_storages: &SnapshotStorage,
        current_ancient: &mut Option<(Slot, Arc<AccountStorageEntry>)>,
        slot: Slot,
    ) -> bool {
        if all_storages.len() != 1 {
            // we are dealing with roots that are more than 1 epoch old. I chose not to support or test the case where we have > 1 append vec per slot.
            // So, such slots will NOT participate in ancient shrinking.
            // since we skipped an ancient append vec, we don't want to append to whatever append vec USED to be the current one
            *current_ancient = None;
            return false;
        }
        let storage = all_storages.first().unwrap();
        let accounts = &storage.accounts;

        // randomly shrink ancient slots
        // this exercises the ancient shrink code more often
        let random_shrink = thread_rng().gen_range(0, 100) == 0 && is_ancient(accounts);

        if is_full_ancient(accounts) || random_shrink {
            if self.is_candidate_for_shrink(storage, true) || random_shrink {
                // we are full, but we are a candidate for shrink, so either append us to the previous append vec
                // or recreate us as a new append vec and eliminate some contents
                info!("ancient_append_vec: shrinking full ancient: {}", slot);
                return true;
            }
            // since we skipped an ancient append vec, we don't want to append to whatever append vec USED to be the current one
            *current_ancient = None;
            return false; // skip this full ancient append vec completely
        }

        if is_ancient(accounts) {
            // this slot is ancient and can become the 'current' ancient for other slots to be squashed into
            *current_ancient = Some((slot, Arc::clone(storage)));
            return false; // we're done with this slot - this slot IS the ancient append vec
        }

        // otherwise, yes, squash this slot into the current ancient append vec or create one at this slot
        true
    }

    /// Combine all account data from storages in 'sorted_slots' into ancient append vecs.
    /// This keeps us from accumulating append vecs in slots older than an epoch.
    fn combine_ancient_slots(&self, sorted_slots: Vec<Slot>) {
        if sorted_slots.is_empty() {
            return;
        }
        let mut guard = None;

        // the ancient append vec currently being written to
        let mut current_ancient = None;
        let mut dropped_roots = vec![];

        // we have to keep track of what pubkeys exist in the current ancient append vec so we can unref correctly
        let mut ancient_pubkeys = HashSet::default();
        let mut ancient_slot_with_pubkeys = None;

        let len = sorted_slots.len();
        for slot in sorted_slots {
            let old_storages =
                match self.get_storages_to_move_to_ancient_append_vec(slot, &mut current_ancient) {
                    Some(old_storages) => old_storages,
                    None => {
                        // nothing to squash for this slot
                        continue;
                    }
                };

            if guard.is_none() {
                // we are now doing interesting work in squashing ancient
                guard = Some(self.active_stats.activate(ActiveStatItem::SquashAncient));
                info!(
                    "ancient_append_vec: combine_ancient_slots first slot: {}, num_roots: {}",
                    slot, len
                );
            }

            // this code is copied from shrink. I would like to combine it into a helper function, but the borrow checker has defeated my efforts so far.
            let GetUniqueAccountsResult {
                stored_accounts,
                original_bytes,
                store_ids: _,
            } = self.get_unique_accounts_from_storages(old_storages.iter());

            // sort by pubkey to keep account index lookups close
            let stored_accounts = {
                let mut stored_accounts = stored_accounts.into_iter().collect::<Vec<_>>();
                stored_accounts.sort_unstable_by(|a, b| a.0.cmp(&b.0));
                stored_accounts
            };

            let mut index_read_elapsed = Measure::start("index_read_elapsed");
            let alive_total_collect = AtomicUsize::new(0);

            let len = stored_accounts.len();
            let alive_accounts_collect = Mutex::new(Vec::with_capacity(len));
            self.shrink_stats
                .accounts_loaded
                .fetch_add(len as u64, Ordering::Relaxed);

            self.thread_pool_clean.install(|| {
                let chunk_size = 50; // # accounts/thread
                let chunks = len / chunk_size + 1;
                (0..chunks).into_par_iter().for_each(|chunk| {
                    let skip = chunk * chunk_size;

                    let mut alive_accounts = Vec::with_capacity(chunk_size);
                    let alive_total = self.load_accounts_index_for_shrink(
                        &stored_accounts[skip..],
                        chunk_size,
                        &mut alive_accounts,
                        None,
                    );

                    // collect
                    alive_accounts_collect
                        .lock()
                        .unwrap()
                        .append(&mut alive_accounts);
                    alive_total_collect.fetch_add(alive_total, Ordering::Relaxed);
                });
            });

            let mut create_and_insert_store_elapsed = 0;

            let alive_accounts = alive_accounts_collect.into_inner().unwrap();
            let alive_total = alive_total_collect.load(Ordering::Relaxed);
            index_read_elapsed.stop();
            let aligned_total: u64 = Self::page_align(alive_total as u64);
            // could follow what shrink does more closely
            if stored_accounts.is_empty() {
                continue; // skipping slot with no useful accounts to write
            }

            let total_starting_accounts = stored_accounts.len();
            let total_accounts_after_shrink = alive_accounts.len();

            let (_, time) = self.maybe_create_ancient_append_vec(&mut current_ancient, slot);
            create_and_insert_store_elapsed += time.as_micros() as u64;
            let (ancient_slot, ancient_store) =
                current_ancient.as_ref().map(|(a, b)| (*a, b)).unwrap();
            let available_bytes = ancient_store.accounts.remaining_bytes();
            let mut start = Measure::start("find_alive_elapsed");
            let to_store = AccountsToStore::new(available_bytes, &alive_accounts, slot);
            start.stop();
            let find_alive_elapsed = start.as_us();

            let mut ids = vec![ancient_store.append_vec_id()];
            // if this slot is not the ancient slot we're writing to, then this root will be dropped
            let mut drop_root = slot != ancient_slot;

            if slot != ancient_slot {
                // we are taking accounts from 'slot' and putting them into 'ancient_slot'
                let (accounts, _hashes) = to_store.get(StorageSelector::Primary);
                if Some(ancient_slot) != ancient_slot_with_pubkeys {
                    // 'ancient_slot_with_pubkeys' is a local, re-used only for the set of slots we're iterating right now.
                    // the first time or when we change to a new ancient append vec, we need to recreate the set of ancient pubkeys here.
                    ancient_slot_with_pubkeys = Some(ancient_slot);
                    ancient_pubkeys = ancient_store
                        .accounts
                        .account_iter()
                        .map(|account| account.meta.pubkey)
                        .collect::<HashSet<_>>();
                }
                // accounts in 'slot' but ALSO already in the ancient append vec at a different slot need to be unref'd since 'slot' is going away
                self.unref_accounts_already_in_storage(accounts, &mut ancient_pubkeys);
            }

            let mut rewrite_elapsed = Measure::start("rewrite_elapsed");
            // write what we can to the current ancient storage
            let mut store_accounts_timing = self.store_ancient_accounts(
                ancient_slot,
                ancient_store,
                &to_store,
                StorageSelector::Primary,
            );

            // handle accounts from 'slot' which did not fit into the current ancient append vec
            if to_store.has_overflow() {
                // we need a new ancient append vec
                let result = self.create_ancient_append_vec(slot);
                create_and_insert_store_elapsed += result.1.as_micros() as u64;
                current_ancient = result.0;
                let (ancient_slot, ancient_store) =
                    current_ancient.as_ref().map(|(a, b)| (*a, b)).unwrap();
                info!(
                    "ancient_append_vec: combine_ancient_slots {}, overflow: {} accounts",
                    slot,
                    to_store.get(StorageSelector::Overflow).0.len()
                );

                ids.push(ancient_store.append_vec_id());
                // if this slot is not the ancient slot we're writing to, then this root will be dropped
                drop_root = slot != ancient_slot;

                // write the rest to the next ancient storage
                let timing = self.store_ancient_accounts(
                    ancient_slot,
                    ancient_store,
                    &to_store,
                    StorageSelector::Overflow,
                );
                store_accounts_timing.store_accounts_elapsed = timing.store_accounts_elapsed;
                store_accounts_timing.update_index_elapsed = timing.update_index_elapsed;
                store_accounts_timing.handle_reclaims_elapsed = timing.handle_reclaims_elapsed;
            }
            rewrite_elapsed.stop();

            let mut start = Measure::start("write_storage_elapsed");
            // Purge old, overwritten storage entries
            let mut dead_storages = vec![];
            self.mark_dirty_dead_stores(slot, &mut dead_storages, |store| {
                ids.contains(&store.append_vec_id())
            });
            start.stop();
            let write_storage_elapsed = start.as_us();

            self.drop_or_recycle_stores(dead_storages);

            if drop_root {
                dropped_roots.push(slot);
            }

            self.shrink_ancient_stats
                .shrink_stats
                .index_read_elapsed
                .fetch_add(index_read_elapsed.as_us(), Ordering::Relaxed);
            self.shrink_ancient_stats
                .shrink_stats
                .create_and_insert_store_elapsed
                .fetch_add(create_and_insert_store_elapsed, Ordering::Relaxed);
            self.shrink_ancient_stats
                .shrink_stats
                .store_accounts_elapsed
                .fetch_add(
                    store_accounts_timing.store_accounts_elapsed,
                    Ordering::Relaxed,
                );
            self.shrink_ancient_stats
                .shrink_stats
                .update_index_elapsed
                .fetch_add(
                    store_accounts_timing.update_index_elapsed,
                    Ordering::Relaxed,
                );
            self.shrink_ancient_stats
                .shrink_stats
                .handle_reclaims_elapsed
                .fetch_add(
                    store_accounts_timing.handle_reclaims_elapsed,
                    Ordering::Relaxed,
                );
            self.shrink_ancient_stats
                .shrink_stats
                .write_storage_elapsed
                .fetch_add(write_storage_elapsed, Ordering::Relaxed);
            self.shrink_ancient_stats
                .shrink_stats
                .rewrite_elapsed
                .fetch_add(rewrite_elapsed.as_us(), Ordering::Relaxed);
            self.shrink_ancient_stats
                .shrink_stats
                .accounts_removed
                .fetch_add(
                    total_starting_accounts - total_accounts_after_shrink,
                    Ordering::Relaxed,
                );
            self.shrink_ancient_stats
                .shrink_stats
                .bytes_removed
                .fetch_add(
                    original_bytes.saturating_sub(aligned_total),
                    Ordering::Relaxed,
                );
            self.shrink_ancient_stats
                .shrink_stats
                .bytes_written
                .fetch_add(aligned_total, Ordering::Relaxed);
            self.shrink_ancient_stats
                .shrink_stats
                .find_alive_elapsed
                .fetch_add(find_alive_elapsed, Ordering::Relaxed);
            self.shrink_ancient_stats
                .shrink_stats
                .num_slots_shrunk
                .fetch_add(1, Ordering::Relaxed);
        }

        if !dropped_roots.is_empty() {
            dropped_roots.iter().for_each(|slot| {
                self.accounts_index
                    .clean_dead_slot(*slot, &mut AccountsIndexRootsStats::default());
                self.bank_hashes.write().unwrap().remove(slot);
                // all storages have been removed from here and recycled or dropped
                assert!(self
                    .storage
                    .map
                    .remove(slot)
                    .unwrap()
                    .1
                    .read()
                    .unwrap()
                    .is_empty());
            });
        }

        self.shrink_ancient_stats.report();
    }

    pub fn shrink_candidate_slots(&self) -> usize {
        let shrink_candidates_slots =
            std::mem::take(&mut *self.shrink_candidate_slots.lock().unwrap());
        if !shrink_candidates_slots.is_empty() {
            self.shrink_ancient_slots();
        }

        let (shrink_slots, shrink_slots_next_batch) = {
            if let AccountShrinkThreshold::TotalSpace { shrink_ratio } = self.shrink_ratio {
                let (shrink_slots, shrink_slots_next_batch) =
                    Self::select_candidates_by_total_usage(&shrink_candidates_slots, shrink_ratio);
                (shrink_slots, Some(shrink_slots_next_batch))
            } else {
                (shrink_candidates_slots, None)
            }
        };

        if shrink_slots.is_empty()
            && shrink_slots_next_batch
                .as_ref()
                .map(|s| s.is_empty())
                .unwrap_or(true)
        {
            return 0;
        }

        let _guard = self.active_stats.activate(ActiveStatItem::Shrink);

        let mut measure_shrink_all_candidates = Measure::start("shrink_all_candidate_slots-ms");
        let num_candidates = shrink_slots.len();
        let shrink_candidates_count: usize = self.thread_pool_clean.install(|| {
            shrink_slots
                .into_par_iter()
                .map(|(slot, slot_shrink_candidates)| {
                    let mut measure = Measure::start("shrink_candidate_slots-ms");
                    self.do_shrink_slot_stores(slot, slot_shrink_candidates.values());
                    measure.stop();
                    inc_new_counter_info!("shrink_candidate_slots-ms", measure.as_ms() as usize);
                    slot_shrink_candidates.len()
                })
                .sum()
        });
        measure_shrink_all_candidates.stop();
        inc_new_counter_info!(
            "shrink_all_candidate_slots-ms",
            measure_shrink_all_candidates.as_ms() as usize
        );
        inc_new_counter_info!("shrink_all_candidate_slots-count", shrink_candidates_count);
        let mut pended_counts: usize = 0;
        if let Some(shrink_slots_next_batch) = shrink_slots_next_batch {
            let mut shrink_slots = self.shrink_candidate_slots.lock().unwrap();
            for (slot, stores) in shrink_slots_next_batch {
                pended_counts += stores.len();
                shrink_slots.entry(slot).or_default().extend(stores);
            }
        }
        inc_new_counter_info!("shrink_pended_stores-count", pended_counts);

        num_candidates
    }

    pub fn shrink_all_slots(&self, is_startup: bool, last_full_snapshot_slot: Option<Slot>) {
        let _guard = self.active_stats.activate(ActiveStatItem::Shrink);
        const DIRTY_STORES_CLEANING_THRESHOLD: usize = 10_000;
        const OUTER_CHUNK_SIZE: usize = 2000;
        if is_startup && self.caching_enabled {
            let slots = self.all_slots_in_storage();
            let threads = num_cpus::get();
            let inner_chunk_size = std::cmp::max(OUTER_CHUNK_SIZE / threads, 1);
            slots.chunks(OUTER_CHUNK_SIZE).for_each(|chunk| {
                chunk.par_chunks(inner_chunk_size).for_each(|slots| {
                    for slot in slots {
                        self.shrink_slot_forced(*slot);
                    }
                });
                if self.dirty_stores.len() > DIRTY_STORES_CLEANING_THRESHOLD {
                    self.clean_accounts(None, is_startup, last_full_snapshot_slot);
                }
            });
        } else {
            for slot in self.all_slots_in_storage() {
                if self.caching_enabled {
                    self.shrink_slot_forced(slot);
                } else {
                    self.do_shrink_slot_forced_v1(slot);
                }
                if self.dirty_stores.len() > DIRTY_STORES_CLEANING_THRESHOLD {
                    self.clean_accounts(None, is_startup, last_full_snapshot_slot);
                }
            }
        }
    }

    pub fn scan_accounts<F, A>(
        &self,
        ancestors: &Ancestors,
        bank_id: BankId,
        scan_func: F,
        config: &ScanConfig,
    ) -> ScanResult<A>
    where
        F: Fn(&mut A, Option<(&Pubkey, AccountSharedData, Slot)>),
        A: Default,
    {
        let mut collector = A::default();

        // This can error out if the slots being scanned over are aborted
        self.accounts_index.scan_accounts(
            ancestors,
            bank_id,
            |pubkey, (account_info, slot)| {
                let account_slot = self
                    .get_account_accessor(slot, pubkey, &account_info.storage_location())
                    .get_loaded_account()
                    .map(|loaded_account| (pubkey, loaded_account.take_account(), slot));
                scan_func(&mut collector, account_slot)
            },
            config,
        )?;

        Ok(collector)
    }

    pub fn unchecked_scan_accounts<F, A>(
        &self,
        metric_name: &'static str,
        ancestors: &Ancestors,
        scan_func: F,
        config: &ScanConfig,
    ) -> A
    where
        F: Fn(&mut A, (&Pubkey, LoadedAccount, Slot)),
        A: Default,
    {
        let mut collector = A::default();
        self.accounts_index.unchecked_scan_accounts(
            metric_name,
            ancestors,
            |pubkey, (account_info, slot)| {
                if let Some(loaded_account) = self
                    .get_account_accessor(slot, pubkey, &account_info.storage_location())
                    .get_loaded_account()
                {
                    scan_func(&mut collector, (pubkey, loaded_account, slot));
                }
            },
            config,
        );
        collector
    }

    /// Only guaranteed to be safe when called from rent collection
    pub fn range_scan_accounts<F, A, R>(
        &self,
        metric_name: &'static str,
        ancestors: &Ancestors,
        range: R,
        config: &ScanConfig,
        scan_func: F,
    ) -> A
    where
        F: Fn(&mut A, Option<(&Pubkey, AccountSharedData, Slot)>),
        A: Default,
        R: RangeBounds<Pubkey> + std::fmt::Debug,
    {
        let mut collector = A::default();
        self.accounts_index.range_scan_accounts(
            metric_name,
            ancestors,
            range,
            config,
            |pubkey, (account_info, slot)| {
                // unlike other scan fns, this is called from Bank::collect_rent_eagerly(),
                // which is on-consensus processing in the banking/replaying stage.
                // This requires infallible and consistent account loading.
                // So, we unwrap Option<LoadedAccount> from get_loaded_account() here.
                // This is safe because this closure is invoked with the account_info,
                // while we lock the index entry at AccountsIndex::do_scan_accounts() ultimately,
                // meaning no other subsystems can invalidate the account_info before making their
                // changes to the index entry.
                // For details, see the comment in retry_to_get_account_accessor()
                if let Some(account_slot) = self
                    .get_account_accessor(slot, pubkey, &account_info.storage_location())
                    .get_loaded_account()
                    .map(|loaded_account| (pubkey, loaded_account.take_account(), slot))
                {
                    scan_func(&mut collector, Some(account_slot))
                }
            },
        );
        collector
    }

    pub fn index_scan_accounts<F, A>(
        &self,
        ancestors: &Ancestors,
        bank_id: BankId,
        index_key: IndexKey,
        scan_func: F,
        config: &ScanConfig,
    ) -> ScanResult<(A, bool)>
    where
        F: Fn(&mut A, Option<(&Pubkey, AccountSharedData, Slot)>),
        A: Default,
    {
        let key = match &index_key {
            IndexKey::ProgramId(key) => key,
            IndexKey::SplTokenMint(key) => key,
            IndexKey::SplTokenOwner(key) => key,
        };
        if !self.account_indexes.include_key(key) {
            // the requested key was not indexed in the secondary index, so do a normal scan
            let used_index = false;
            let scan_result = self.scan_accounts(ancestors, bank_id, scan_func, config)?;
            return Ok((scan_result, used_index));
        }

        let mut collector = A::default();
        self.accounts_index.index_scan_accounts(
            ancestors,
            bank_id,
            index_key,
            |pubkey, (account_info, slot)| {
                let account_slot = self
                    .get_account_accessor(slot, pubkey, &account_info.storage_location())
                    .get_loaded_account()
                    .map(|loaded_account| (pubkey, loaded_account.take_account(), slot));
                scan_func(&mut collector, account_slot)
            },
            config,
        )?;
        let used_index = true;
        Ok((collector, used_index))
    }

    /// Scan a specific slot through all the account storage in parallel
    pub fn scan_account_storage<R, B>(
        &self,
        slot: Slot,
        cache_map_func: impl Fn(LoadedAccount) -> Option<R> + Sync,
        storage_scan_func: impl Fn(&B, LoadedAccount) + Sync,
    ) -> ScanStorageResult<R, B>
    where
        R: Send,
        B: Send + Default + Sync,
    {
        if let Some(slot_cache) = self.accounts_cache.slot_cache(slot) {
            // If we see the slot in the cache, then all the account information
            // is in this cached slot
            if slot_cache.len() > SCAN_SLOT_PAR_ITER_THRESHOLD {
                ScanStorageResult::Cached(self.thread_pool.install(|| {
                    slot_cache
                        .par_iter()
                        .filter_map(|cached_account| {
                            cache_map_func(LoadedAccount::Cached(Cow::Borrowed(
                                cached_account.value(),
                            )))
                        })
                        .collect()
                }))
            } else {
                ScanStorageResult::Cached(
                    slot_cache
                        .iter()
                        .filter_map(|cached_account| {
                            cache_map_func(LoadedAccount::Cached(Cow::Borrowed(
                                cached_account.value(),
                            )))
                        })
                        .collect(),
                )
            }
        } else {
            let retval = B::default();
            // If the slot is not in the cache, then all the account information must have
            // been flushed. This is guaranteed because we only remove the rooted slot from
            // the cache *after* we've finished flushing in `flush_slot_cache`.
            let storage_maps: Vec<Arc<AccountStorageEntry>> = self
                .storage
                .get_slot_storage_entries(slot)
                .unwrap_or_default();
            self.thread_pool.install(|| {
                storage_maps.par_iter().for_each(|storage| {
                    storage.accounts.account_iter().for_each(|account| {
                        storage_scan_func(&retval, LoadedAccount::Stored(account))
                    })
                });
            });

            ScanStorageResult::Stored(retval)
        }
    }

    pub fn set_hash(&self, slot: Slot, parent_slot: Slot) {
        let mut bank_hashes = self.bank_hashes.write().unwrap();
        if bank_hashes.get(&slot).is_some() {
            error!(
                "set_hash: already exists; multiple forks with shared slot {} as child (parent: {})!?",
                slot, parent_slot,
            );
            return;
        }

        let new_hash_info = BankHashInfo {
            hash: Hash::default(),
            snapshot_hash: Hash::default(),
            stats: BankHashStats::default(),
        };
        bank_hashes.insert(slot, new_hash_info);
    }

    pub fn load(
        &self,
        ancestors: &Ancestors,
        pubkey: &Pubkey,
        load_hint: LoadHint,
    ) -> Option<(AccountSharedData, Slot)> {
        self.do_load(ancestors, pubkey, None, load_hint)
    }

    pub fn load_account_into_read_cache(&self, ancestors: &Ancestors, pubkey: &Pubkey) {
        self.do_load_with_populate_read_cache(ancestors, pubkey, None, LoadHint::Unspecified, true);
    }

    pub fn load_with_fixed_root(
        &self,
        ancestors: &Ancestors,
        pubkey: &Pubkey,
    ) -> Option<(AccountSharedData, Slot)> {
        self.load(ancestors, pubkey, LoadHint::FixedMaxRoot)
    }

    pub fn load_without_fixed_root(
        &self,
        ancestors: &Ancestors,
        pubkey: &Pubkey,
    ) -> Option<(AccountSharedData, Slot)> {
        self.load(ancestors, pubkey, LoadHint::Unspecified)
    }

    fn read_index_for_accessor_or_load_slow<'a>(
        &'a self,
        ancestors: &Ancestors,
        pubkey: &'a Pubkey,
        max_root: Option<Slot>,
        clone_in_lock: bool,
    ) -> Option<(Slot, StorageLocation, Option<LoadedAccountAccessor<'a>>)> {
        let (lock, index) = match self.accounts_index.get(pubkey, Some(ancestors), max_root) {
            AccountIndexGetResult::Found(lock, index) => (lock, index),
            // we bail out pretty early for missing.
            AccountIndexGetResult::NotFound => {
                return None;
            }
        };

        let slot_list = lock.slot_list();
        let (slot, info) = slot_list[index];
        let storage_location = info.storage_location();
        let some_from_slow_path = if clone_in_lock {
            // the fast path must have failed.... so take the slower approach
            // of copying potentially large Account::data inside the lock.

            // calling check_and_get_loaded_account is safe as long as we're guaranteed to hold
            // the lock during the time and there should be no purge thanks to alive ancestors
            // held by our caller.
            Some(self.get_account_accessor(slot, pubkey, &storage_location))
        } else {
            None
        };

        Some((slot, storage_location, some_from_slow_path))
        // `lock` is dropped here rather pretty quickly with clone_in_lock = false,
        // so the entry could be raced for mutation by other subsystems,
        // before we actually provision an account data for caller's use from now on.
        // This is traded for less contention and resultant performance, introducing fair amount of
        // delicate handling in retry_to_get_account_accessor() below ;)
        // you're warned!
    }

    fn retry_to_get_account_accessor<'a>(
        &'a self,
        mut slot: Slot,
        mut storage_location: StorageLocation,
        ancestors: &'a Ancestors,
        pubkey: &'a Pubkey,
        max_root: Option<Slot>,
        load_hint: LoadHint,
    ) -> Option<(LoadedAccountAccessor<'a>, Slot)> {
        // Happy drawing time! :)
        //
        // Reader                               | Accessed data source for cached/stored
        // -------------------------------------+----------------------------------
        // R1 read_index_for_accessor_or_load_slow()| cached/stored: index
        //          |                           |
        //        <(store_id, offset, ..)>      |
        //          V                           |
        // R2 retry_to_get_account_accessor()/  | cached: map of caches & entry for (slot, pubkey)
        //        get_account_accessor()        | stored: map of stores
        //          |                           |
        //        <Accessor>                    |
        //          V                           |
        // R3 check_and_get_loaded_account()/   | cached: N/A (note: basically noop unwrap)
        //        get_loaded_account()          | stored: store's entry for slot
        //          |                           |
        //        <LoadedAccount>               |
        //          V                           |
        // R4 take_account()                    | cached/stored: entry of cache/storage for (slot, pubkey)
        //          |                           |
        //        <AccountSharedData>           |
        //          V                           |
        //    Account!!                         V
        //
        // Flusher                              | Accessed data source for cached/stored
        // -------------------------------------+----------------------------------
        // F1 flush_slot_cache()                | N/A
        //          |                           |
        //          V                           |
        // F2 store_accounts_frozen()/          | map of stores (creates new entry)
        //        write_accounts_to_storage()   |
        //          |                           |
        //          V                           |
        // F3 store_accounts_frozen()/          | index
        //        update_index()                | (replaces existing store_id, offset in caches)
        //          |                           |
        //          V                           |
        // F4 accounts_cache.remove_slot()      | map of caches (removes old entry)
        //                                      V
        //
        // Remarks for flusher: So, for any reading operations, it's a race condition where F4 happens
        // between R1 and R2. In that case, retrying from R1 is safu because F3 should have
        // been occurred.
        //
        // Shrinker                             | Accessed data source for stored
        // -------------------------------------+----------------------------------
        // S1 do_shrink_slot_stores()           | N/A
        //          |                           |
        //          V                           |
        // S2 store_accounts_frozen()/          | map of stores (creates new entry)
        //        write_accounts_to_storage()   |
        //          |                           |
        //          V                           |
        // S3 store_accounts_frozen()/          | index
        //        update_index()                | (replaces existing store_id, offset in stores)
        //          |                           |
        //          V                           |
        // S4 do_shrink_slot_stores()/          | map of stores (removes old entry)
        //        dead_storages
        //
        // Remarks for shrinker: So, for any reading operations, it's a race condition
        // where S4 happens between R1 and R2. In that case, retrying from R1 is safu because S3 should have
        // been occurred, and S3 atomically replaced the index accordingly.
        //
        // Cleaner                              | Accessed data source for stored
        // -------------------------------------+----------------------------------
        // C1 clean_accounts()                  | N/A
        //          |                           |
        //          V                           |
        // C2 clean_accounts()/                 | index
        //        purge_keys_exact()            | (removes existing store_id, offset for stores)
        //          |                           |
        //          V                           |
        // C3 clean_accounts()/                 | map of stores (removes old entry)
        //        handle_reclaims()             |
        //
        // Remarks for cleaner: So, for any reading operations, it's a race condition
        // where C3 happens between R1 and R2. In that case, retrying from R1 is safu.
        // In that case, None would be returned while bailing out at R1.
        //
        // Purger                                 | Accessed data source for cached/stored
        // ---------------------------------------+----------------------------------
        // P1 purge_slot()                        | N/A
        //          |                             |
        //          V                             |
        // P2 purge_slots_from_cache_and_store()  | map of caches/stores (removes old entry)
        //          |                             |
        //          V                             |
        // P3 purge_slots_from_cache_and_store()/ | index
        //       purge_slot_cache()/              |
        //          purge_slot_cache_pubkeys()    | (removes existing store_id, offset for cache)
        //       purge_slot_storage()/            |
        //          purge_keys_exact()            | (removes accounts index entries)
        //          handle_reclaims()             | (removes storage entries)
        //      OR                                |
        //    clean_accounts()/                   |
        //        clean_accounts_older_than_root()| (removes existing store_id, offset for stores)
        //                                        V
        //
        // Remarks for purger: So, for any reading operations, it's a race condition
        // where P2 happens between R1 and R2. In that case, retrying from R1 is safu.
        // In that case, we may bail at index read retry when P3 hasn't been run

        #[cfg(test)]
        {
            // Give some time for cache flushing to occur here for unit tests
            sleep(Duration::from_millis(self.load_delay));
        }

