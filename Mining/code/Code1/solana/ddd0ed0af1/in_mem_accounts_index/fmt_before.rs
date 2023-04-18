    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

pub enum InsertNewEntryResults {
    DidNotExist,
    ExistedNewEntryZeroLamports,
    ExistedNewEntryNonZeroLamports,
}

/// result from scanning in-mem index during flush
struct FlushScanResult<T> {
    /// pubkeys whose age indicates they may be evicted now, pending further checks.
    evictions_age_possible: Vec<(Pubkey, Option<AccountMapEntry<T>>)>,
    /// pubkeys chosen to evict based on random eviction
    evictions_random: Vec<(Pubkey, Option<AccountMapEntry<T>>)>,
}

#[allow(dead_code)] // temporary during staging
impl<T: IndexValue> InMemAccountsIndex<T> {
    pub fn new(storage: &Arc<BucketMapHolder<T>>, bin: usize) -> Self {
        Self {
            map_internal: RwLock::default(),
            storage: Arc::clone(storage),
            bin,
            bucket: storage
                .disk
                .as_ref()
                .map(|disk| disk.get_bucket_from_index(bin))
                .map(Arc::clone),
            cache_ranges_held: CacheRangesHeld::default(),
            stop_evictions_changes: AtomicU64::default(),
            stop_evictions: AtomicU64::default(),
            flushing_active: AtomicBool::default(),
            // initialize this to max, to make it clear we have not flushed at age 0, the starting age
            last_age_flushed: AtomicU8::new(Age::MAX),
        }
    }

    /// true if this bucket needs to call flush for the current age
    /// we need to scan each bucket once per value of age
    fn get_should_age(&self, age: Age) -> bool {
        let last_age_flushed = self.last_age_flushed();
        last_age_flushed != age
    }

    /// called after flush scans this bucket at the current age
    fn set_has_aged(&self, age: Age) {
        self.last_age_flushed.store(age, Ordering::Relaxed);
        self.storage.bucket_flushed_at_current_age();
    }

    fn last_age_flushed(&self) -> Age {
        self.last_age_flushed.load(Ordering::Relaxed)
    }

    fn map(&self) -> &RwLock<HashMap<Pubkey, AccountMapEntry<T>>> {
        &self.map_internal
    }

    /// Release entire in-mem hashmap to free all memory associated with it.
    /// Idea is that during startup we needed a larger map than we need during runtime.
    /// When using disk-buckets, in-mem index grows over time with dynamic use and then shrinks, in theory back to 0.
    pub fn shrink_to_fit(&self) {
        // shrink_to_fit could be quite expensive on large map sizes, which 'no disk buckets' could produce, so avoid shrinking in case we end up here
        if self.storage.is_disk_index_enabled() {
            self.map_internal.write().unwrap().shrink_to_fit();
        }
    }

    pub fn items<R>(&self, range: &R) -> Vec<(K, AccountMapEntry<T>)>
    where
        R: RangeBounds<Pubkey> + std::fmt::Debug,
    {
        let m = Measure::start("items");
        self.hold_range_in_memory(range, true);
        let map = self.map().read().unwrap();
        let mut result = Vec::with_capacity(map.len());
        map.iter().for_each(|(k, v)| {
            if range.contains(k) {
                result.push((*k, Arc::clone(v)));
            }
        });
        self.hold_range_in_memory(range, false);
        Self::update_stat(&self.stats().items, 1);
        Self::update_time_stat(&self.stats().items_us, m);
        result
    }

    // only called in debug code paths
    pub fn keys(&self) -> Vec<Pubkey> {
        Self::update_stat(&self.stats().keys, 1);
        // easiest implementation is to load evrything from disk into cache and return the keys
        self.start_stop_evictions(true);
        self.put_range_in_cache(&None::<&RangeInclusive<Pubkey>>);
        let keys = self.map().read().unwrap().keys().cloned().collect();
        self.start_stop_evictions(false);
        keys
    }

    fn load_from_disk(&self, pubkey: &Pubkey) -> Option<(SlotList<T>, RefCount)> {
        self.bucket.as_ref().and_then(|disk| {
            let m = Measure::start("load_disk_found_count");
            let entry_disk = disk.read_value(pubkey);
            match &entry_disk {
                Some(_) => {
                    Self::update_time_stat(&self.stats().load_disk_found_us, m);
                    Self::update_stat(&self.stats().load_disk_found_count, 1);
                }
                None => {
                    Self::update_time_stat(&self.stats().load_disk_missing_us, m);
                    Self::update_stat(&self.stats().load_disk_missing_count, 1);
                }
            }
            entry_disk
        })
    }

    fn load_account_entry_from_disk(&self, pubkey: &Pubkey) -> Option<AccountMapEntry<T>> {
        let entry_disk = self.load_from_disk(pubkey)?; // returns None if not on disk

        Some(self.disk_to_cache_entry(entry_disk.0, entry_disk.1))
    }

    /// lookup 'pubkey' by only looking in memory. Does not look on disk.
    /// callback is called whether pubkey is found or not
    fn get_only_in_mem<RT>(
        &self,
        pubkey: &K,
        callback: impl for<'a> FnOnce(Option<&'a AccountMapEntry<T>>) -> RT,
    ) -> RT {
        let mut found = true;
        let mut m = Measure::start("get");
        let result = {
            let map = self.map().read().unwrap();
            let result = map.get(pubkey);
            m.stop();

            callback(if let Some(entry) = result {
                entry.set_age(self.storage.future_age_to_flush());
                Some(entry)
            } else {
                drop(map);
                found = false;
                None
            })
        };

        let stats = self.stats();
        let (count, time) = if found {
            (&stats.gets_from_mem, &stats.get_mem_us)
        } else {
            (&stats.gets_missing, &stats.get_missing_us)
        };
        Self::update_stat(time, m.as_us());
        Self::update_stat(count, 1);

        result
    }

    /// lookup 'pubkey' in index (in mem or on disk)
    pub fn get(&self, pubkey: &K) -> Option<AccountMapEntry<T>> {
        self.get_internal(pubkey, |entry| (true, entry.map(Arc::clone)))
    }

    /// lookup 'pubkey' in index (in_mem or disk).
    /// call 'callback' whether found or not
    pub(crate) fn get_internal<RT>(
        &self,
        pubkey: &K,
        // return true if item should be added to in_mem cache
        callback: impl for<'a> FnOnce(Option<&AccountMapEntry<T>>) -> (bool, RT),
    ) -> RT {
        self.get_only_in_mem(pubkey, |entry| {
            if let Some(entry) = entry {
                entry.set_age(self.storage.future_age_to_flush());
                callback(Some(entry)).1
            } else {
                // not in cache, look on disk
                let stats = &self.stats();
                let disk_entry = self.load_account_entry_from_disk(pubkey);
                if disk_entry.is_none() {
                    return callback(None).1;
                }
                let disk_entry = disk_entry.unwrap();
                let mut map = self.map().write().unwrap();
                let entry = map.entry(*pubkey);
                match entry {
                    Entry::Occupied(occupied) => callback(Some(occupied.get())).1,
                    Entry::Vacant(vacant) => {
                        let (add_to_cache, rt) = callback(Some(&disk_entry));

                        if add_to_cache {
                            stats.insert_or_delete_mem(true, self.bin);
                            vacant.insert(disk_entry);
                        }
                        rt
                    }
                }
            }
        })
    }

    fn remove_if_slot_list_empty_value(&self, slot_list: SlotSlice<T>) -> bool {
        if slot_list.is_empty() {
            self.stats().insert_or_delete(false, self.bin);
            true
        } else {
            false
        }
    }

    fn delete_disk_key(&self, pubkey: &Pubkey) {
        if let Some(disk) = self.bucket.as_ref() {
            disk.delete_key(pubkey)
        }
    }

    fn remove_if_slot_list_empty_entry(&self, entry: Entry<K, AccountMapEntry<T>>) -> bool {
        match entry {
            Entry::Occupied(occupied) => {
                let result =
                    self.remove_if_slot_list_empty_value(&occupied.get().slot_list.read().unwrap());
                if result {
                    // note there is a potential race here that has existed.
                    // if someone else holds the arc,
                    //  then they think the item is still in the index and can make modifications.
                    // We have to have a write lock to the map here, which means nobody else can get
                    //  the arc, but someone may already have retreived a clone of it.
                    // account index in_mem flushing is one such possibility
                    self.delete_disk_key(occupied.key());
                    self.stats().insert_or_delete_mem(false, self.bin);
                    occupied.remove();
                }
                result
            }
            Entry::Vacant(vacant) => {
                // not in cache, look on disk
                let entry_disk = self.load_from_disk(vacant.key());
                match entry_disk {
                    Some(entry_disk) => {
                        // on disk
                        if self.remove_if_slot_list_empty_value(&entry_disk.0) {
                            // not in cache, but on disk, so just delete from disk
                            self.delete_disk_key(vacant.key());
                            true
                        } else {
                            // could insert into cache here, but not required for correctness and value is unclear
                            false
                        }
                    }
                    None => false, // not in cache or on disk
                }
            }
        }
    }

    // If the slot list for pubkey exists in the index and is empty, remove the index entry for pubkey and return true.
    // Return false otherwise.
    pub fn remove_if_slot_list_empty(&self, pubkey: Pubkey) -> bool {
        let mut m = Measure::start("entry");
        let mut map = self.map().write().unwrap();
        let entry = map.entry(pubkey);
        m.stop();
        let found = matches!(entry, Entry::Occupied(_));
        let result = self.remove_if_slot_list_empty_entry(entry);
        drop(map);

        self.update_entry_stats(m, found);
        result
    }

    pub fn slot_list_mut<RT>(
        &self,
        pubkey: &Pubkey,
        user: impl for<'a> FnOnce(&mut RwLockWriteGuard<'a, SlotList<T>>) -> RT,
    ) -> Option<RT> {
        self.get_internal(pubkey, |entry| {
            (
                true,
                entry.map(|entry| {
                    let result = user(&mut entry.slot_list.write().unwrap());
                    entry.set_dirty(true);
                    result
                }),
            )
        })
    }
