    fn new(total_count: &'a AtomicU64, report_delay_secs: u64, ultimate_count: u64) -> Self {
        Self {
            last_update: Instant::now(),
            my_last_report_count: 0,
            total_count,
            report_delay_secs,
            first_caller: false,
            ultimate_count,
        }
    }
    fn report(&mut self, my_current_count: u64) {
        let now = Instant::now();
        if now.duration_since(self.last_update).as_secs() >= self.report_delay_secs {
            let my_total_newly_processed_slots_since_last_report =
                my_current_count - self.my_last_report_count;

            self.my_last_report_count = my_current_count;
            let previous_total_processed_slots_across_all_threads = self.total_count.fetch_add(
                my_total_newly_processed_slots_since_last_report,
                Ordering::Relaxed,
            );
            self.first_caller =
                self.first_caller || 0 == previous_total_processed_slots_across_all_threads;
            if self.first_caller {
                info!(
                    "generating index: {}/{} slots...",
                    previous_total_processed_slots_across_all_threads
                        + my_total_newly_processed_slots_since_last_report,
                    self.ultimate_count
                );
            }
            self.last_update = now;
        }
    }
}

/// An offset into the AccountsDb::storage vector
pub type AtomicAppendVecId = AtomicU32;
pub type AppendVecId = u32;
pub type SnapshotStorage = Vec<Arc<AccountStorageEntry>>;
pub type SnapshotStorages = Vec<SnapshotStorage>;

// Each slot has a set of storage entries.
pub(crate) type SlotStores = Arc<RwLock<HashMap<AppendVecId, Arc<AccountStorageEntry>>>>;

type AccountSlots = HashMap<Pubkey, HashSet<Slot>>;
type AppendVecOffsets = HashMap<AppendVecId, HashSet<usize>>;
type ReclaimResult = (AccountSlots, AppendVecOffsets);
type ShrinkCandidates = HashMap<Slot, HashMap<AppendVecId, Arc<AccountStorageEntry>>>;

trait Versioned {
    fn version(&self) -> u64;
}

impl Versioned for (u64, Hash) {
    fn version(&self) -> u64 {
        self.0
    }
}

impl Versioned for (u64, AccountInfo) {
    fn version(&self) -> u64 {
        self.0
    }
}

// Some hints for applicability of additional sanity checks for the do_load fast-path;
// Slower fallback code path will be taken if the fast path has failed over the retry
// threshold, regardless of these hints. Also, load cannot fail not-deterministically
// even under very rare circumstances, unlike previously did allow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadHint {
    // Caller hints that it's loading transactions for a block which is
    // descended from the current root, and at the tip of its fork.
    // Thereby, further this assumes AccountIndex::max_root should not increase
    // during this load, meaning there should be no squash.
    // Overall, this enables us to assert!() strictly while running the fast-path for
    // account loading, while maintaining the determinism of account loading and resultant
    // transaction execution thereof.
    FixedMaxRoot,
    // Caller can't hint the above safety assumption. Generally RPC and miscellaneous
    // other call-site falls into this category. The likelihood of slower path is slightly
    // increased as well.
    Unspecified,
}

#[derive(Debug)]
pub enum LoadedAccountAccessor<'a> {
    // StoredAccountMeta can't be held directly here due to its lifetime dependency to
    // AccountStorageEntry
    Stored(Option<(Arc<AccountStorageEntry>, usize)>),
    // None value in Cached variant means the cache was flushed
    Cached(Option<Cow<'a, CachedAccount>>),
}

mod geyser_plugin_utils;

impl<'a> LoadedAccountAccessor<'a> {
    fn check_and_get_loaded_account(&mut self) -> LoadedAccount {
        // all of these following .expect() and .unwrap() are like serious logic errors,
        // ideal for representing this as rust type system....

        match self {
            LoadedAccountAccessor::Cached(None) | LoadedAccountAccessor::Stored(None) => {
                panic!("Should have already been taken care of when creating this LoadedAccountAccessor");
            }
            LoadedAccountAccessor::Cached(Some(_cached_account)) => {
                // Cached(Some(x)) variant always produces `Some` for get_loaded_account() since
                // it just returns the inner `x` without additional fetches
                self.get_loaded_account().unwrap()
            }
            LoadedAccountAccessor::Stored(Some(_maybe_storage_entry)) => {
                // If we do find the storage entry, we can guarantee that the storage entry is
                // safe to read from because we grabbed a reference to the storage entry while it
                // was still in the storage map. This means even if the storage entry is removed
                // from the storage map after we grabbed the storage entry, the recycler should not
                // reset the storage entry until we drop the reference to the storage entry.
                self.get_loaded_account()
                    .expect("If a storage entry was found in the storage map, it must not have been reset yet")
            }
        }
    }

    fn get_loaded_account(&mut self) -> Option<LoadedAccount> {
        match self {
            LoadedAccountAccessor::Cached(cached_account) => {
                let cached_account: Cow<'a, CachedAccount> = cached_account.take().expect(
                    "Cache flushed/purged should be handled before trying to fetch account",
                );
                Some(LoadedAccount::Cached(cached_account))
            }
            LoadedAccountAccessor::Stored(maybe_storage_entry) => {
                // storage entry may not be present if slot was cleaned up in
                // between reading the accounts index and calling this function to
                // get account meta from the storage entry here
                maybe_storage_entry
                    .as_ref()
                    .and_then(|(storage_entry, offset)| {
                        storage_entry
                            .get_stored_account_meta(*offset)
                            .map(LoadedAccount::Stored)
                    })
            }
        }
    }
}

pub enum LoadedAccount<'a> {
    Stored(StoredAccountMeta<'a>),
    Cached(Cow<'a, CachedAccount>),
}

impl<'a> LoadedAccount<'a> {
    pub fn loaded_hash(&self) -> Hash {
        match self {
            LoadedAccount::Stored(stored_account_meta) => *stored_account_meta.hash,
            LoadedAccount::Cached(cached_account) => cached_account.hash(),
        }
    }

    pub fn pubkey(&self) -> &Pubkey {
        match self {
            LoadedAccount::Stored(stored_account_meta) => &stored_account_meta.meta.pubkey,
            LoadedAccount::Cached(cached_account) => cached_account.pubkey(),
        }
    }

    pub fn write_version(&self) -> StoredMetaWriteVersion {
        match self {
            LoadedAccount::Stored(stored_account_meta) => stored_account_meta.meta.write_version,
            LoadedAccount::Cached(_) => CACHE_VIRTUAL_WRITE_VERSION,
        }
    }

    pub fn compute_hash(&self, slot: Slot, pubkey: &Pubkey) -> Hash {
        match self {
            LoadedAccount::Stored(stored_account_meta) => AccountsDb::hash_account(
                slot,
                stored_account_meta,
                &stored_account_meta.meta.pubkey,
            ),
            LoadedAccount::Cached(cached_account) => {
                AccountsDb::hash_account(slot, &cached_account.account, pubkey)
            }
        }
    }

    pub fn stored_size(&self) -> usize {
        match self {
            LoadedAccount::Stored(stored_account_meta) => stored_account_meta.stored_size,
            LoadedAccount::Cached(_) => CACHE_VIRTUAL_STORED_SIZE as usize,
        }
    }

    pub fn take_account(self) -> AccountSharedData {
        match self {
            LoadedAccount::Stored(stored_account_meta) => stored_account_meta.clone_account(),
            LoadedAccount::Cached(cached_account) => match cached_account {
                Cow::Owned(cached_account) => cached_account.account.clone(),
                Cow::Borrowed(cached_account) => cached_account.account.clone(),
            },
        }
    }

    pub fn is_cached(&self) -> bool {
        match self {
            LoadedAccount::Stored(_) => false,
            LoadedAccount::Cached(_) => true,
        }
    }
}

impl<'a> ReadableAccount for LoadedAccount<'a> {
    fn lamports(&self) -> u64 {
        match self {
            LoadedAccount::Stored(stored_account_meta) => stored_account_meta.account_meta.lamports,
            LoadedAccount::Cached(cached_account) => cached_account.account.lamports(),
        }
    }

    fn data(&self) -> &[u8] {
        match self {
            LoadedAccount::Stored(stored_account_meta) => stored_account_meta.data,
            LoadedAccount::Cached(cached_account) => cached_account.account.data(),
        }
    }
    fn owner(&self) -> &Pubkey {
        match self {
            LoadedAccount::Stored(stored_account_meta) => &stored_account_meta.account_meta.owner,
            LoadedAccount::Cached(cached_account) => cached_account.account.owner(),
        }
    }
    fn executable(&self) -> bool {
        match self {
            LoadedAccount::Stored(stored_account_meta) => {
                stored_account_meta.account_meta.executable
            }
            LoadedAccount::Cached(cached_account) => cached_account.account.executable(),
        }
    }
    fn rent_epoch(&self) -> Epoch {
        match self {
            LoadedAccount::Stored(stored_account_meta) => {
                stored_account_meta.account_meta.rent_epoch
            }
            LoadedAccount::Cached(cached_account) => cached_account.account.rent_epoch(),
        }
    }
    fn to_account_shared_data(&self) -> AccountSharedData {
        match self {
            LoadedAccount::Stored(_stored_account_meta) => AccountSharedData::create(
                self.lamports(),
                self.data().to_vec(),
                *self.owner(),
                self.executable(),
                self.rent_epoch(),
            ),
            // clone here to prevent data copy
            LoadedAccount::Cached(cached_account) => cached_account.account.clone(),
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct AccountStorage {
    pub map: DashMap<Slot, SlotStores>,
}

impl AccountStorage {
    fn get_account_storage_entry(
        &self,
        slot: Slot,
        store_id: AppendVecId,
    ) -> Option<Arc<AccountStorageEntry>> {
        self.get_slot_stores(slot)
            .and_then(|storage_map| storage_map.read().unwrap().get(&store_id).cloned())
    }

    pub fn get_slot_stores(&self, slot: Slot) -> Option<SlotStores> {
        self.map.get(&slot).map(|result| result.value().clone())
    }

    fn get_slot_storage_entries(&self, slot: Slot) -> Option<Vec<Arc<AccountStorageEntry>>> {
        self.get_slot_stores(slot)
            .map(|res| res.read().unwrap().values().cloned().collect())
    }

    fn slot_store_count(&self, slot: Slot, store_id: AppendVecId) -> Option<usize> {
        self.get_account_storage_entry(slot, store_id)
            .map(|store| store.count())
    }

    fn all_slots(&self) -> Vec<Slot> {
        self.map.iter().map(|iter_item| *iter_item.key()).collect()
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Deserialize, Serialize, AbiExample, AbiEnumVisitor)]
pub enum AccountStorageStatus {
    Available = 0,
    Full = 1,
    Candidate = 2,
}

impl Default for AccountStorageStatus {
    fn default() -> Self {
        Self::Available
    }
}

#[derive(Debug)]
pub enum BankHashVerificationError {
    MismatchedAccountHash,
    MismatchedBankHash,
    MissingBankHash,
    MismatchedTotalLamports(u64, u64),
}

#[derive(Default)]
struct CleanKeyTimings {
    collect_delta_keys_us: u64,
    delta_insert_us: u64,
    hashset_to_vec_us: u64,
    dirty_store_processing_us: u64,
    delta_key_count: u64,
    dirty_pubkeys_count: u64,
}

/// Persistent storage structure holding the accounts
#[derive(Debug)]
pub struct AccountStorageEntry {
    pub(crate) id: AtomicAppendVecId,

    pub(crate) slot: AtomicU64,

    /// storage holding the accounts
    pub(crate) accounts: AppendVec,

    /// Keeps track of the number of accounts stored in a specific AppendVec.
    ///  This is periodically checked to reuse the stores that do not have
    ///  any accounts in it
    /// status corresponding to the storage, lets us know that
    ///  the append_vec, once maxed out, then emptied, can be reclaimed
    count_and_status: RwLock<(usize, AccountStorageStatus)>,

    /// This is the total number of accounts stored ever since initialized to keep
    /// track of lifetime count of all store operations. And this differs from
    /// count_and_status in that this field won't be decremented.
    ///
    /// This is used as a rough estimate for slot shrinking. As such a relaxed
    /// use case, this value ARE NOT strictly synchronized with count_and_status!
    approx_store_count: AtomicUsize,

    alive_bytes: AtomicUsize,
}

impl AccountStorageEntry {
    pub fn new(path: &Path, slot: Slot, id: AppendVecId, file_size: u64) -> Self {
        let tail = AppendVec::file_name(slot, id);
        let path = Path::new(path).join(tail);
        let accounts = AppendVec::new(&path, true, file_size as usize);

        Self {
            id: AtomicAppendVecId::new(id),
            slot: AtomicU64::new(slot),
            accounts,
            count_and_status: RwLock::new((0, AccountStorageStatus::Available)),
            approx_store_count: AtomicUsize::new(0),
            alive_bytes: AtomicUsize::new(0),
        }
    }

    pub(crate) fn new_existing(
        slot: Slot,
        id: AppendVecId,
        accounts: AppendVec,
        num_accounts: usize,
    ) -> Self {
        Self {
            id: AtomicAppendVecId::new(id),
            slot: AtomicU64::new(slot),
            accounts,
            count_and_status: RwLock::new((0, AccountStorageStatus::Available)),
            approx_store_count: AtomicUsize::new(num_accounts),
            alive_bytes: AtomicUsize::new(0),
        }
    }

    pub fn set_status(&self, mut status: AccountStorageStatus) {
        let mut count_and_status = self.count_and_status.write().unwrap();

        let count = count_and_status.0;

        if status == AccountStorageStatus::Full && count == 0 {
            // this case arises when the append_vec is full (store_ptrs fails),
            //  but all accounts have already been removed from the storage
            //
            // the only time it's safe to call reset() on an append_vec is when
            //  every account has been removed
            //          **and**
            //  the append_vec has previously been completely full
            //
            self.accounts.reset();
            status = AccountStorageStatus::Available;
        }

        *count_and_status = (count, status);
    }

    pub fn recycle(&self, slot: Slot, id: AppendVecId) {
        let mut count_and_status = self.count_and_status.write().unwrap();
        self.accounts.reset();
        *count_and_status = (0, AccountStorageStatus::Available);
        self.slot.store(slot, Ordering::Release);
        self.id.store(id, Ordering::Release);
        self.approx_store_count.store(0, Ordering::Relaxed);
        self.alive_bytes.store(0, Ordering::Release);
    }

    pub fn status(&self) -> AccountStorageStatus {
        self.count_and_status.read().unwrap().1
    }

    pub fn count(&self) -> usize {
        self.count_and_status.read().unwrap().0
    }

    pub fn approx_stored_count(&self) -> usize {
        self.approx_store_count.load(Ordering::Relaxed)
    }

    pub fn alive_bytes(&self) -> usize {
        self.alive_bytes.load(Ordering::SeqCst)
    }

    pub fn written_bytes(&self) -> u64 {
        self.accounts.len() as u64
    }

    pub fn total_bytes(&self) -> u64 {
        self.accounts.capacity()
    }

    pub fn has_accounts(&self) -> bool {
        self.count() > 0
    }

    pub fn slot(&self) -> Slot {
        self.slot.load(Ordering::Acquire)
    }

    pub fn append_vec_id(&self) -> AppendVecId {
        self.id.load(Ordering::Acquire)
    }

    pub fn flush(&self) -> Result<(), IoError> {
        self.accounts.flush()
    }

    fn get_stored_account_meta(&self, offset: usize) -> Option<StoredAccountMeta> {
        Some(self.accounts.get_account(offset)?.0)
    }

    fn add_account(&self, num_bytes: usize) {
        let mut count_and_status = self.count_and_status.write().unwrap();
        *count_and_status = (count_and_status.0 + 1, count_and_status.1);
        self.approx_store_count.fetch_add(1, Ordering::Relaxed);
        self.alive_bytes.fetch_add(num_bytes, Ordering::SeqCst);
    }

    fn try_available(&self) -> bool {
        let mut count_and_status = self.count_and_status.write().unwrap();
        let (count, status) = *count_and_status;

        if status == AccountStorageStatus::Available {
            *count_and_status = (count, AccountStorageStatus::Candidate);
            true
        } else {
            false
        }
    }

    pub fn all_accounts(&self) -> Vec<StoredAccountMeta> {
        self.accounts.accounts(0)
    }

    fn remove_account(&self, num_bytes: usize, reset_accounts: bool) -> usize {
        let mut count_and_status = self.count_and_status.write().unwrap();
        let (mut count, mut status) = *count_and_status;

        if count == 1 && status == AccountStorageStatus::Full && reset_accounts {
            // this case arises when we remove the last account from the
            //  storage, but we've learned from previous write attempts that
            //  the storage is full
            //
            // the only time it's safe to call reset() on an append_vec is when
            //  every account has been removed
            //          **and**
            //  the append_vec has previously been completely full
            //
            // otherwise, the storage may be in flight with a store()
            //   call
            self.accounts.reset();
            status = AccountStorageStatus::Available;
        }

        // Some code path is removing accounts too many; this may result in an
        // unintended reveal of old state for unrelated accounts.
        assert!(
            count > 0,
            "double remove of account in slot: {}/store: {}!!",
            self.slot(),
            self.append_vec_id(),
        );

        self.alive_bytes.fetch_sub(num_bytes, Ordering::SeqCst);
        count -= 1;
        *count_and_status = (count, status);
        count
    }

    pub fn get_path(&self) -> PathBuf {
        self.accounts.get_path()
    }
}

pub fn get_temp_accounts_paths(count: u32) -> IoResult<(Vec<TempDir>, Vec<PathBuf>)> {
    let temp_dirs: IoResult<Vec<TempDir>> = (0..count).map(|_| TempDir::new()).collect();
    let temp_dirs = temp_dirs?;
    let paths: Vec<PathBuf> = temp_dirs.iter().map(|t| t.path().to_path_buf()).collect();
    Ok((temp_dirs, paths))
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, AbiExample)]
pub struct BankHashStats {
    pub num_updated_accounts: u64,
    pub num_removed_accounts: u64,
    pub num_lamports_stored: u64,
    pub total_data_len: u64,
    pub num_executable_accounts: u64,
}

impl BankHashStats {
    pub fn update<T: ReadableAccount + ZeroLamport>(&mut self, account: &T) {
        if account.is_zero_lamport() {
            self.num_removed_accounts += 1;
        } else {
            self.num_updated_accounts += 1;
        }
        self.total_data_len = self
            .total_data_len
            .wrapping_add(account.data().len() as u64);
        if account.executable() {
            self.num_executable_accounts += 1;
        }
        self.num_lamports_stored = self.num_lamports_stored.wrapping_add(account.lamports());
    }

    pub fn merge(&mut self, other: &BankHashStats) {
        self.num_updated_accounts += other.num_updated_accounts;
        self.num_removed_accounts += other.num_removed_accounts;
        self.total_data_len = self.total_data_len.wrapping_add(other.total_data_len);
        self.num_lamports_stored = self
            .num_lamports_stored
            .wrapping_add(other.num_lamports_stored);
        self.num_executable_accounts += other.num_executable_accounts;
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, AbiExample)]
pub struct BankHashInfo {
    pub hash: Hash,
    pub snapshot_hash: Hash,
    pub stats: BankHashStats,
}

#[derive(Default)]
pub struct StoreAccountsTiming {
    store_accounts_elapsed: u64,
    update_index_elapsed: u64,
    handle_reclaims_elapsed: u64,
}

#[derive(Debug, Default)]
struct RecycleStores {
    entries: Vec<(Instant, Arc<AccountStorageEntry>)>,
    total_bytes: u64,
}

// 30 min should be enough to be certain there won't be any prospective recycle uses for given
// store entry
// That's because it already processed ~2500 slots and ~25 passes of AccountsBackgroundService
pub const EXPIRATION_TTL_SECONDS: u64 = 1800;

impl RecycleStores {
    fn add_entry(&mut self, new_entry: Arc<AccountStorageEntry>) {
        self.total_bytes += new_entry.total_bytes();
        self.entries.push((Instant::now(), new_entry))
    }

    fn iter(&self) -> std::slice::Iter<(Instant, Arc<AccountStorageEntry>)> {
        self.entries.iter()
    }

    fn add_entries(&mut self, new_entries: Vec<Arc<AccountStorageEntry>>) {
        self.total_bytes += new_entries.iter().map(|e| e.total_bytes()).sum::<u64>();
        let now = Instant::now();
        for new_entry in new_entries {
            self.entries.push((now, new_entry));
        }
    }

    fn expire_old_entries(&mut self) -> Vec<Arc<AccountStorageEntry>> {
        let mut expired = vec![];
        let now = Instant::now();
        let mut expired_bytes = 0;
        self.entries.retain(|(recycled_time, entry)| {
            if now.duration_since(*recycled_time).as_secs() > EXPIRATION_TTL_SECONDS {
                if Arc::strong_count(entry) >= 2 {
                    warn!(
                        "Expiring still in-use recycled StorageEntry anyway...: id: {} slot: {}",
                        entry.append_vec_id(),
                        entry.slot(),
                    );
                }
                expired_bytes += entry.total_bytes();
                expired.push(entry.clone());
                false
            } else {
                true
            }
        });

        self.total_bytes -= expired_bytes;

        expired
    }

    fn remove_entry(&mut self, index: usize) -> Arc<AccountStorageEntry> {
        let (_added_time, removed_entry) = self.entries.swap_remove(index);
        self.total_bytes -= removed_entry.total_bytes();
        removed_entry
    }

    fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

/// Removing unrooted slots in Accounts Background Service needs to be synchronized with flushing
/// slots from the Accounts Cache.  This keeps track of those slots and the Mutex + Condvar for
/// synchronization.
#[derive(Debug, Default)]
struct RemoveUnrootedSlotsSynchronization {
    // slots being flushed from the cache or being purged
    slots_under_contention: Mutex<HashSet<Slot>>,
    signal: Condvar,
}

type AccountInfoAccountsIndex = AccountsIndex<AccountInfo>;

// This structure handles the load/store of the accounts
#[derive(Debug)]
pub struct AccountsDb {
    /// Keeps tracks of index into AppendVec on a per slot basis
    pub accounts_index: AccountInfoAccountsIndex,

    /// slot that is one epoch older than the highest slot where accounts hash calculation has completed
    pub accounts_hash_complete_one_epoch_old: RwLock<Slot>,

    /// true iff rent exempt accounts are not rewritten in their normal rent collection slot
    pub skip_rewrites: bool,

    /// true iff we want to squash old append vecs together into 'ancient append vecs'
    pub ancient_append_vecs: bool,

    /// true iff we want to skip the initial hash calculation on startup
    pub skip_initial_hash_calc: bool,

    pub storage: AccountStorage,

    pub accounts_cache: AccountsCache,

    write_cache_limit_bytes: Option<u64>,

    sender_bg_hasher: Option<Sender<CachedAccount>>,
    read_only_accounts_cache: ReadOnlyAccountsCache,

    recycle_stores: RwLock<RecycleStores>,

    /// distribute the accounts across storage lists
    pub next_id: AtomicAppendVecId,

    /// Set of shrinkable stores organized by map of slot to append_vec_id
    pub shrink_candidate_slots: Mutex<ShrinkCandidates>,

    /// Legacy shrink slots to support non-cached code-path.
    pub shrink_candidate_slots_v1: Mutex<Vec<Slot>>,

    pub(crate) write_version: AtomicU64,

    /// Set of storage paths to pick from
    pub(crate) paths: Vec<PathBuf>,

    accounts_hash_cache_path: PathBuf,

    // used by tests
    // holds this until we are dropped
    #[allow(dead_code)]
    temp_accounts_hash_cache_path: Option<TempDir>,

    pub shrink_paths: RwLock<Option<Vec<PathBuf>>>,

    /// Directory of paths this accounts_db needs to hold/remove
    #[allow(dead_code)]
    pub(crate) temp_paths: Option<Vec<TempDir>>,

    /// Starting file size of appendvecs
    file_size: u64,

    /// Thread pool used for par_iter
    pub thread_pool: ThreadPool,

    pub thread_pool_clean: ThreadPool,

    /// Number of append vecs to create to maximize parallelism when scanning
    /// the accounts
    min_num_stores: usize,

    pub bank_hashes: RwLock<HashMap<Slot, BankHashInfo>>,

    pub stats: AccountsStats,

    clean_accounts_stats: CleanAccountsStats,

    // Stats for purges called outside of clean_accounts()
    external_purge_slots_stats: PurgeStats,

    shrink_stats: ShrinkStats,

    shrink_ancient_stats: ShrinkAncientStats,

    pub cluster_type: Option<ClusterType>,

    pub account_indexes: AccountSecondaryIndexes,

    pub caching_enabled: bool,

    /// Set of unique keys per slot which is used
    /// to drive clean_accounts
    /// Generated by get_accounts_delta_hash
    uncleaned_pubkeys: DashMap<Slot, Vec<Pubkey>>,

    #[cfg(test)]
    load_delay: u64,

    #[cfg(test)]
    load_limit: AtomicU64,

    /// true if drop_callback is attached to the bank.
    is_bank_drop_callback_enabled: AtomicBool,

    pub startup_verification_complete: Arc<AtomicBool>,

    /// Set of slots currently being flushed by `flush_slot_cache()` or removed
    /// by `remove_unrooted_slot()`. Used to ensure `remove_unrooted_slots(slots)`
    /// can safely clear the set of unrooted slots `slots`.
    remove_unrooted_slots_synchronization: RemoveUnrootedSlotsSynchronization,

    shrink_ratio: AccountShrinkThreshold,

    /// Set of stores which are recently rooted or had accounts removed
    /// such that potentially a 0-lamport account update could be present which
    /// means we can remove the account from the index entirely.
    dirty_stores: DashMap<(Slot, AppendVecId), Arc<AccountStorageEntry>>,

    /// Zero-lamport accounts that are *not* purged during clean because they need to stay alive
    /// for incremental snapshot support.
    zero_lamport_accounts_to_purge_after_full_snapshot: DashSet<(Slot, Pubkey)>,

    /// GeyserPlugin accounts update notifier
    accounts_update_notifier: Option<AccountsUpdateNotifier>,

    filler_accounts_config: FillerAccountsConfig,
    pub filler_account_suffix: Option<Pubkey>,

    active_stats: ActiveStats,

    /// number of filler accounts to add for each slot
    pub filler_accounts_per_slot: AtomicU64,

    /// number of slots remaining where filler accounts should be added
    pub filler_account_slots_remaining: AtomicU64,

    // # of passes should be a function of the total # of accounts that are active.
    // higher passes = slower total time, lower dynamic memory usage
    // lower passes = faster total time, higher dynamic memory usage
    // passes=2 cuts dynamic memory usage in approximately half.
    pub num_hash_scan_passes: Option<usize>,

    /// Used to disable logging dead slots during removal.
    /// allow disabling noisy log
    pub(crate) log_dead_slots: AtomicBool,
}

#[derive(Debug, Default)]
pub struct AccountsStats {
    delta_hash_scan_time_total_us: AtomicU64,
    delta_hash_accumulate_time_total_us: AtomicU64,
    delta_hash_num: AtomicU64,

    last_store_report: AtomicInterval,
    store_hash_accounts: AtomicU64,
    calc_stored_meta: AtomicU64,
    store_accounts: AtomicU64,
    store_update_index: AtomicU64,
    store_handle_reclaims: AtomicU64,
    store_append_accounts: AtomicU64,
    pub stakes_cache_check_and_store_us: AtomicU64,
    store_find_store: AtomicU64,
    store_num_accounts: AtomicU64,
    store_total_data: AtomicU64,
    recycle_store_count: AtomicU64,
    create_store_count: AtomicU64,
    store_get_slot_store: AtomicU64,
    store_find_existing: AtomicU64,
    dropped_stores: AtomicU64,
    store_uncleaned_update: AtomicU64,
}

#[derive(Debug, Default)]
pub(crate) struct PurgeStats {
    last_report: AtomicInterval,
    safety_checks_elapsed: AtomicU64,
    remove_cache_elapsed: AtomicU64,
    remove_storage_entries_elapsed: AtomicU64,
    drop_storage_entries_elapsed: AtomicU64,
    num_cached_slots_removed: AtomicUsize,
    num_stored_slots_removed: AtomicUsize,
    total_removed_storage_entries: AtomicUsize,
    total_removed_cached_bytes: AtomicU64,
    total_removed_stored_bytes: AtomicU64,
    recycle_stores_write_elapsed: AtomicU64,
    scan_storages_elapsed: AtomicU64,
    purge_accounts_index_elapsed: AtomicU64,
    handle_reclaims_elapsed: AtomicU64,
}

impl PurgeStats {
    fn report(&self, metric_name: &'static str, report_interval_ms: Option<u64>) {
        let should_report = report_interval_ms
            .map(|report_interval_ms| self.last_report.should_update(report_interval_ms))
            .unwrap_or(true);

        if should_report {
            datapoint_info!(
                metric_name,
                (
                    "safety_checks_elapsed",
                    self.safety_checks_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "remove_cache_elapsed",
                    self.remove_cache_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "remove_storage_entries_elapsed",
                    self.remove_storage_entries_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "drop_storage_entries_elapsed",
                    self.drop_storage_entries_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "num_cached_slots_removed",
                    self.num_cached_slots_removed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "num_stored_slots_removed",
                    self.num_stored_slots_removed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "total_removed_storage_entries",
                    self.total_removed_storage_entries
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "total_removed_cached_bytes",
                    self.total_removed_cached_bytes.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "total_removed_stored_bytes",
                    self.total_removed_stored_bytes.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "recycle_stores_write_elapsed",
                    self.recycle_stores_write_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "scan_storages_elapsed",
                    self.scan_storages_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "purge_accounts_index_elapsed",
                    self.purge_accounts_index_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "handle_reclaims_elapsed",
                    self.handle_reclaims_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
            );
        }
    }
}

#[derive(Debug, Default)]
struct FlushStats {
    num_flushed: usize,
    num_purged: usize,
    total_size: u64,
}

#[derive(Debug, Default)]
struct LatestAccountsIndexRootsStats {
    roots_len: AtomicUsize,
    historical_roots_len: AtomicUsize,
    uncleaned_roots_len: AtomicUsize,
    previous_uncleaned_roots_len: AtomicUsize,
    roots_range: AtomicU64,
    rooted_cleaned_count: AtomicUsize,
    unrooted_cleaned_count: AtomicUsize,
    clean_unref_from_storage_us: AtomicU64,
    clean_dead_slot_us: AtomicU64,
}

impl LatestAccountsIndexRootsStats {
    fn update(&self, accounts_index_roots_stats: &AccountsIndexRootsStats) {
        if let Some(value) = accounts_index_roots_stats.roots_len {
            self.roots_len.store(value, Ordering::Relaxed);
        }
        if let Some(value) = accounts_index_roots_stats.uncleaned_roots_len {
            self.uncleaned_roots_len.store(value, Ordering::Relaxed);
        }
        if let Some(value) = accounts_index_roots_stats.previous_uncleaned_roots_len {
            self.previous_uncleaned_roots_len
                .store(value, Ordering::Relaxed);
        }
        if let Some(value) = accounts_index_roots_stats.historical_roots_len {
            self.historical_roots_len.store(value, Ordering::Relaxed);
        }
        if let Some(value) = accounts_index_roots_stats.roots_range {
            self.roots_range.store(value, Ordering::Relaxed);
        }
        self.rooted_cleaned_count.fetch_add(
            accounts_index_roots_stats.rooted_cleaned_count,
            Ordering::Relaxed,
        );
        self.unrooted_cleaned_count.fetch_add(
            accounts_index_roots_stats.unrooted_cleaned_count,
            Ordering::Relaxed,
        );
        self.clean_unref_from_storage_us.fetch_add(
            accounts_index_roots_stats.clean_unref_from_storage_us,
            Ordering::Relaxed,
        );
        self.clean_dead_slot_us.fetch_add(
            accounts_index_roots_stats.clean_dead_slot_us,
            Ordering::Relaxed,
        );
    }

    fn report(&self) {
        datapoint_info!(
            "accounts_index_roots_len",
            (
                "roots_len",
                self.roots_len.load(Ordering::Relaxed) as i64,
                i64
            ),
            (
                "historical_roots_len",
                self.historical_roots_len.load(Ordering::Relaxed) as i64,
                i64
            ),
            (
                "uncleaned_roots_len",
                self.uncleaned_roots_len.load(Ordering::Relaxed) as i64,
                i64
            ),
            (
                "previous_uncleaned_roots_len",
                self.previous_uncleaned_roots_len.load(Ordering::Relaxed) as i64,
                i64
            ),
            (
                "roots_range_width",
                self.roots_range.load(Ordering::Relaxed) as i64,
                i64
            ),
            (
                "unrooted_cleaned_count",
                self.unrooted_cleaned_count.swap(0, Ordering::Relaxed) as i64,
                i64
            ),
            (
                "rooted_cleaned_count",
                self.rooted_cleaned_count.swap(0, Ordering::Relaxed) as i64,
                i64
            ),
            (
                "clean_unref_from_storage_us",
                self.clean_unref_from_storage_us.swap(0, Ordering::Relaxed) as i64,
                i64
            ),
            (
                "clean_dead_slot_us",
                self.clean_dead_slot_us.swap(0, Ordering::Relaxed) as i64,
                i64
            ),
        );

        // Don't need to reset since this tracks the latest updates, not a cumulative total
    }
}

#[derive(Debug, Default)]
struct CleanAccountsStats {
    purge_stats: PurgeStats,
    latest_accounts_index_roots_stats: LatestAccountsIndexRootsStats,

    // stats held here and reported by clean_accounts
    clean_old_root_us: AtomicU64,
    clean_old_root_reclaim_us: AtomicU64,
    reset_uncleaned_roots_us: AtomicU64,
    remove_dead_accounts_remove_us: AtomicU64,
    remove_dead_accounts_shrink_us: AtomicU64,
    clean_stored_dead_slots_us: AtomicU64,
}

impl CleanAccountsStats {
    fn report(&self) {
        self.purge_stats.report("clean_purge_slots_stats", None);
        self.latest_accounts_index_roots_stats.report();
    }
}

#[derive(Debug, Default)]
struct ShrinkAncientStats {
    shrink_stats: ShrinkStats,
}

#[derive(Debug, Default)]
struct ShrinkStats {
    last_report: AtomicInterval,
    num_slots_shrunk: AtomicUsize,
    storage_read_elapsed: AtomicU64,
    index_read_elapsed: AtomicU64,
    find_alive_elapsed: AtomicU64,
    create_and_insert_store_elapsed: AtomicU64,
    store_accounts_elapsed: AtomicU64,
    update_index_elapsed: AtomicU64,
    handle_reclaims_elapsed: AtomicU64,
    write_storage_elapsed: AtomicU64,
    rewrite_elapsed: AtomicU64,
    drop_storage_entries_elapsed: AtomicU64,
    recycle_stores_write_elapsed: AtomicU64,
    accounts_removed: AtomicUsize,
    bytes_removed: AtomicU64,
    bytes_written: AtomicU64,
    skipped_shrink: AtomicU64,
    dead_accounts: AtomicU64,
    alive_accounts: AtomicU64,
    accounts_loaded: AtomicU64,
}

impl ShrinkStats {
    fn report(&self) {
        if self.last_report.should_update(1000) {
            datapoint_info!(
                "shrink_stats",
                (
                    "num_slots_shrunk",
                    self.num_slots_shrunk.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "storage_read_elapsed",
                    self.storage_read_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "index_read_elapsed",
                    self.index_read_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "find_alive_elapsed",
                    self.find_alive_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "create_and_insert_store_elapsed",
                    self.create_and_insert_store_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "store_accounts_elapsed",
                    self.store_accounts_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "update_index_elapsed",
                    self.update_index_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "handle_reclaims_elapsed",
                    self.handle_reclaims_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "write_storage_elapsed",
                    self.write_storage_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "rewrite_elapsed",
                    self.rewrite_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "drop_storage_entries_elapsed",
                    self.drop_storage_entries_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "recycle_stores_write_time",
                    self.recycle_stores_write_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "accounts_removed",
                    self.accounts_removed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "bytes_removed",
                    self.bytes_removed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "bytes_written",
                    self.bytes_written.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "skipped_shrink",
                    self.skipped_shrink.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "alive_accounts",
                    self.alive_accounts.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "dead_accounts",
                    self.dead_accounts.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "accounts_loaded",
                    self.accounts_loaded.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
            );
        }
    }
}

impl ShrinkAncientStats {
    fn report(&self) {
        if self.shrink_stats.last_report.should_update(1000) {
            datapoint_info!(
                "shrink_ancient_stats",
                (
                    "num_slots_shrunk",
                    self.shrink_stats
                        .num_slots_shrunk
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "storage_read_elapsed",
                    self.shrink_stats
                        .storage_read_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "index_read_elapsed",
                    self.shrink_stats
                        .index_read_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "find_alive_elapsed",
                    self.shrink_stats
                        .find_alive_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "create_and_insert_store_elapsed",
                    self.shrink_stats
                        .create_and_insert_store_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "store_accounts_elapsed",
                    self.shrink_stats
                        .store_accounts_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "update_index_elapsed",
                    self.shrink_stats
                        .update_index_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "handle_reclaims_elapsed",
                    self.shrink_stats
                        .handle_reclaims_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "write_storage_elapsed",
                    self.shrink_stats
                        .write_storage_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "rewrite_elapsed",
                    self.shrink_stats.rewrite_elapsed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "drop_storage_entries_elapsed",
                    self.shrink_stats
                        .drop_storage_entries_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "recycle_stores_write_time",
                    self.shrink_stats
                        .recycle_stores_write_elapsed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "accounts_removed",
                    self.shrink_stats
                        .accounts_removed
                        .swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "bytes_removed",
                    self.shrink_stats.bytes_removed.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "bytes_written",
                    self.shrink_stats.bytes_written.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "skipped_shrink",
                    self.shrink_stats.skipped_shrink.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "alive_accounts",
                    self.shrink_stats.alive_accounts.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "dead_accounts",
                    self.shrink_stats.dead_accounts.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
                (
                    "accounts_loaded",
                    self.shrink_stats.accounts_loaded.swap(0, Ordering::Relaxed) as i64,
                    i64
                ),
            );
        }
    }
}

pub fn quarter_thread_count() -> usize {
    std::cmp::max(2, num_cpus::get() / 4)
}

pub fn make_min_priority_thread_pool() -> ThreadPool {
    // Use lower thread count to reduce priority.
    let num_threads = quarter_thread_count();
    rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("solana-cleanup-accounts-{}", i))
        .num_threads(num_threads)
        .build()
        .unwrap()
}

#[cfg(all(test, RUSTC_WITH_SPECIALIZATION))]
impl solana_frozen_abi::abi_example::AbiExample for AccountsDb {
    fn example() -> Self {
        let accounts_db = AccountsDb::new_single_for_tests();
        let key = Pubkey::default();
        let some_data_len = 5;
        let some_slot: Slot = 0;
        let account = AccountSharedData::new(1, some_data_len, &key);
        accounts_db.store_uncached(some_slot, &[(&key, &account)]);
        accounts_db.add_root(0);

        accounts_db
    }
}

impl<'a> ZeroLamport for StoredAccountMeta<'a> {
    fn is_zero_lamport(&self) -> bool {
        self.lamports() == 0
    }
}

impl<'a> ReadableAccount for StoredAccountMeta<'a> {
    fn lamports(&self) -> u64 {
        self.account_meta.lamports
    }
    fn data(&self) -> &[u8] {
        self.data
    }
    fn owner(&self) -> &Pubkey {
        &self.account_meta.owner
    }
    fn executable(&self) -> bool {
        self.account_meta.executable
    }
    fn rent_epoch(&self) -> Epoch {
        self.account_meta.rent_epoch
    }
}

struct IndexAccountMapEntry<'a> {
    pub write_version: StoredMetaWriteVersion,
    pub store_id: AppendVecId,
    pub stored_account: StoredAccountMeta<'a>,
}

type GenerateIndexAccountsMap<'a> = HashMap<Pubkey, IndexAccountMapEntry<'a>>;

/// called on a struct while scanning append vecs
trait AppendVecScan: Send + Sync + Clone {
    /// return true if this pubkey should be included
    fn filter(&mut self, pubkey: &Pubkey) -> bool;
    /// set current slot of the scan
    fn set_slot(&mut self, slot: Slot);
    /// found `account` in the append vec
    fn found_account(&mut self, account: &LoadedAccount);
    /// scanning is done
    fn scanning_complete(self) -> BinnedHashData;
    /// initialize accumulator
    fn init_accum(&mut self, count: usize);
    fn get_accum(&mut self) -> BinnedHashData;
    fn set_accum(&mut self, accum: BinnedHashData);
}

#[derive(Clone)]
/// state to keep while scanning append vec accounts for hash calculation
/// These would have been captured in a fn from within the scan function.
/// Some of these are constant across all pubkeys, some are constant across a slot.
/// Some could be unique per pubkey.
struct ScanState<'a, T: Fn(Slot) -> Option<Slot> + Sync + Send + Clone> {
    /// slot we're currently scanning
    current_slot: Slot,
    /// accumulated results
    accum: BinnedHashData,
    /// max slot (inclusive) that we're calculating accounts hash on
    max_slot_info: SlotInfoInEpoch,
    bin_calculator: &'a PubkeyBinCalculator24,
    bin_range: &'a Range<usize>,
    config: &'a CalcAccountsHashConfig<'a>,
    mismatch_found: Arc<AtomicU64>,
    stats: &'a crate::accounts_hash::HashStats,
    find_unskipped_slot: &'a T,
    filler_account_suffix: Option<&'a Pubkey>,
    range: usize,
    sort_time: Arc<AtomicU64>,
    pubkey_to_bin_index: usize,
}

impl<'a, T: Fn(Slot) -> Option<Slot> + Sync + Send + Clone> AppendVecScan for ScanState<'a, T> {
    fn set_slot(&mut self, slot: Slot) {
        self.current_slot = slot;
    }
    fn filter(&mut self, pubkey: &Pubkey) -> bool {
        self.pubkey_to_bin_index = self.bin_calculator.bin_from_pubkey(pubkey);
        self.bin_range.contains(&self.pubkey_to_bin_index)
    }
    fn init_accum(&mut self, count: usize) {
        if self.accum.is_empty() {
            self.accum.append(&mut vec![Vec::new(); count]);
        }
    }
    fn found_account(&mut self, loaded_account: &LoadedAccount) {
        let pubkey = loaded_account.pubkey();
        assert!(self.bin_range.contains(&self.pubkey_to_bin_index)); // get rid of this once we have confidence

        // when we are scanning with bin ranges, we don't need to use exact bin numbers. Subtract to make first bin we care about at index 0.
        self.pubkey_to_bin_index -= self.bin_range.start;

        let raw_lamports = loaded_account.lamports();
        let zero_raw_lamports = raw_lamports == 0;
        let balance = if zero_raw_lamports {
            crate::accounts_hash::ZERO_RAW_LAMPORTS_SENTINEL
        } else {
            raw_lamports
        };

        let loaded_hash = loaded_account.loaded_hash();
        let new_hash = ExpectedRentCollection::maybe_rehash_skipped_rewrite(
            loaded_account,
            &loaded_hash,
            pubkey,
            self.current_slot,
            self.config.epoch_schedule,
            self.config.rent_collector,
            self.stats,
            &self.max_slot_info,
            self.find_unskipped_slot,
            self.filler_account_suffix,
        );
        let loaded_hash = new_hash.unwrap_or(loaded_hash);

        let source_item = CalculateHashIntermediate::new(loaded_hash, balance, *pubkey);

        if self.config.check_hash
            && !AccountsDb::is_filler_account_helper(pubkey, self.filler_account_suffix)
        {
            // this will not be supported anymore
            let computed_hash = loaded_account.compute_hash(self.current_slot, pubkey);
            if computed_hash != source_item.hash {
                info!(
                    "hash mismatch found: computed: {}, loaded: {}, pubkey: {}",
                    computed_hash, source_item.hash, pubkey
                );
                self.mismatch_found.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.init_accum(self.range);
        self.accum[self.pubkey_to_bin_index].push(source_item);
    }
    fn scanning_complete(self) -> BinnedHashData {
        let (result, timing) = AccountsDb::sort_slot_storage_scan(self.accum);
        self.sort_time.fetch_add(timing, Ordering::Relaxed);
        result
    }
    fn get_accum(&mut self) -> BinnedHashData {
        std::mem::take(&mut self.accum)
    }
    fn set_accum(&mut self, accum: BinnedHashData) {
        self.accum = accum;
    }
}

impl AccountsDb {
    pub fn default_for_tests() -> Self {
        Self::default_with_accounts_index(AccountInfoAccountsIndex::default_for_tests(), None, None)
    }

    /// return (num_hash_scan_passes, bins_per_pass)
    fn bins_per_pass(num_hash_scan_passes: Option<usize>) -> (usize, usize) {
        let num_hash_scan_passes = num_hash_scan_passes.unwrap_or(NUM_SCAN_PASSES_DEFAULT);
        let bins_per_pass = PUBKEY_BINS_FOR_CALCULATING_HASHES / num_hash_scan_passes;
        assert!(
            num_hash_scan_passes <= PUBKEY_BINS_FOR_CALCULATING_HASHES,
            "num_hash_scan_passes must be <= {}",
            PUBKEY_BINS_FOR_CALCULATING_HASHES
        );
        assert_eq!(
            bins_per_pass * num_hash_scan_passes,
            PUBKEY_BINS_FOR_CALCULATING_HASHES
        ); // evenly divisible

        (num_hash_scan_passes, bins_per_pass)
    }

    fn default_with_accounts_index(
        accounts_index: AccountInfoAccountsIndex,
        accounts_hash_cache_path: Option<PathBuf>,
        num_hash_scan_passes: Option<usize>,
    ) -> Self {
        let num_threads = get_thread_count();
        const MAX_READ_ONLY_CACHE_DATA_SIZE: usize = 200_000_000;

        let mut temp_accounts_hash_cache_path = None;
        let accounts_hash_cache_path = accounts_hash_cache_path.unwrap_or_else(|| {
            temp_accounts_hash_cache_path = Some(TempDir::new().unwrap());
            temp_accounts_hash_cache_path
                .as_ref()
                .unwrap()
                .path()
                .to_path_buf()
        });

        let mut bank_hashes = HashMap::new();
        bank_hashes.insert(0, BankHashInfo::default());

        // validate inside here
        Self::bins_per_pass(num_hash_scan_passes);

        // Increase the stack for accounts threads
        // rayon needs a lot of stack
        const ACCOUNTS_STACK_SIZE: usize = 8 * 1024 * 1024;

        // this will be live shortly
        // for now, this check occurs at startup, so it must always be true
        let startup_verification_complete = Arc::new(AtomicBool::new(true));

        AccountsDb {
            startup_verification_complete,
            filler_accounts_per_slot: AtomicU64::default(),
            filler_account_slots_remaining: AtomicU64::default(),
            active_stats: ActiveStats::default(),
            accounts_hash_complete_one_epoch_old: RwLock::default(),
            skip_rewrites: false,
            skip_initial_hash_calc: false,
            ancient_append_vecs: false,
            accounts_index,
            storage: AccountStorage::default(),
            accounts_cache: AccountsCache::default(),
            sender_bg_hasher: None,
            read_only_accounts_cache: ReadOnlyAccountsCache::new(MAX_READ_ONLY_CACHE_DATA_SIZE),
            recycle_stores: RwLock::new(RecycleStores::default()),
            uncleaned_pubkeys: DashMap::new(),
            next_id: AtomicAppendVecId::new(0),
            shrink_candidate_slots_v1: Mutex::new(Vec::new()),
            shrink_candidate_slots: Mutex::new(HashMap::new()),
            write_cache_limit_bytes: None,
            write_version: AtomicU64::new(0),
            paths: vec![],
            accounts_hash_cache_path,
            temp_accounts_hash_cache_path,
            shrink_paths: RwLock::new(None),
            temp_paths: None,
            file_size: DEFAULT_FILE_SIZE,
            thread_pool: rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .thread_name(|i| format!("solana-db-accounts-{}", i))
                .stack_size(ACCOUNTS_STACK_SIZE)
                .build()
                .unwrap(),
            thread_pool_clean: make_min_priority_thread_pool(),
            min_num_stores: num_threads,
            bank_hashes: RwLock::new(bank_hashes),
            external_purge_slots_stats: PurgeStats::default(),
            clean_accounts_stats: CleanAccountsStats::default(),
            shrink_stats: ShrinkStats::default(),
            shrink_ancient_stats: ShrinkAncientStats::default(),
            stats: AccountsStats::default(),
            cluster_type: None,
            account_indexes: AccountSecondaryIndexes::default(),
            caching_enabled: false,
            #[cfg(test)]
            load_delay: u64::default(),
            #[cfg(test)]
            load_limit: AtomicU64::default(),
            is_bank_drop_callback_enabled: AtomicBool::default(),
            remove_unrooted_slots_synchronization: RemoveUnrootedSlotsSynchronization::default(),
            shrink_ratio: AccountShrinkThreshold::default(),
            dirty_stores: DashMap::default(),
            zero_lamport_accounts_to_purge_after_full_snapshot: DashSet::default(),
            accounts_update_notifier: None,
            filler_accounts_config: FillerAccountsConfig::default(),
            filler_account_suffix: None,
            num_hash_scan_passes,
            log_dead_slots: AtomicBool::new(true),
        }
    }

    pub fn new_for_tests(paths: Vec<PathBuf>, cluster_type: &ClusterType) -> Self {
        AccountsDb::new_with_config(
            paths,
            cluster_type,
            AccountSecondaryIndexes::default(),
            false,
            AccountShrinkThreshold::default(),
            Some(ACCOUNTS_DB_CONFIG_FOR_TESTING),
            None,
        )
    }

    pub fn new_for_tests_with_caching(paths: Vec<PathBuf>, cluster_type: &ClusterType) -> Self {
        AccountsDb::new_with_config(
            paths,
            cluster_type,
            AccountSecondaryIndexes::default(),
            true,
            AccountShrinkThreshold::default(),
            Some(ACCOUNTS_DB_CONFIG_FOR_TESTING),
            None,
        )
    }

    pub fn new_with_config(
        paths: Vec<PathBuf>,
        cluster_type: &ClusterType,
        account_indexes: AccountSecondaryIndexes,
        caching_enabled: bool,
        shrink_ratio: AccountShrinkThreshold,
        mut accounts_db_config: Option<AccountsDbConfig>,
        accounts_update_notifier: Option<AccountsUpdateNotifier>,
    ) -> Self {
        let accounts_index =
            AccountsIndex::new(accounts_db_config.as_mut().and_then(|x| x.index.take()));
        let accounts_hash_cache_path = accounts_db_config
            .as_ref()
            .and_then(|x| x.accounts_hash_cache_path.clone());

        let filler_accounts_config = accounts_db_config
            .as_ref()
            .map(|config| config.filler_accounts_config)
            .unwrap_or_default();
        let skip_rewrites = accounts_db_config
            .as_ref()
            .map(|config| config.skip_rewrites)
            .unwrap_or_default();
        let skip_initial_hash_calc = accounts_db_config
            .as_ref()
            .map(|config| config.skip_initial_hash_calc)
            .unwrap_or_default();

        let ancient_append_vecs = accounts_db_config
            .as_ref()
            .map(|config| config.ancient_append_vecs)
            .unwrap_or_default();

        let filler_account_suffix = if filler_accounts_config.count > 0 {
            Some(solana_sdk::pubkey::new_rand())
        } else {
            None
        };
        let paths_is_empty = paths.is_empty();
        let mut new = Self {
            paths,
            skip_rewrites,
            skip_initial_hash_calc,
            ancient_append_vecs,
            cluster_type: Some(*cluster_type),
            account_indexes,
            caching_enabled,
            shrink_ratio,
            accounts_update_notifier,
            filler_accounts_config,
            filler_account_suffix,
            write_cache_limit_bytes: accounts_db_config
                .as_ref()
                .and_then(|x| x.write_cache_limit_bytes),
            ..Self::default_with_accounts_index(
                accounts_index,
                accounts_hash_cache_path,
                accounts_db_config
                    .as_ref()
                    .and_then(|cfg| cfg.hash_calc_num_passes),
            )
        };
        if paths_is_empty {
            // Create a temporary set of accounts directories, used primarily
            // for testing
            let (temp_dirs, paths) = get_temp_accounts_paths(DEFAULT_NUM_DIRS).unwrap();
            new.accounts_update_notifier = None;
            new.paths = paths;
            new.temp_paths = Some(temp_dirs);
        };

        new.start_background_hasher();
        {
            for path in new.paths.iter() {
                std::fs::create_dir_all(path).expect("Create directory failed.");
            }
        }
        new
    }

    /// Gradual means filler accounts will be added over the course of an epoch, during cache flush.
    /// This is in contrast to adding all the filler accounts immediately before the validator starts.
    fn init_gradual_filler_accounts(&self, slots_per_epoch: Slot) {
        let count = self.filler_accounts_config.count;
        if count > 0 {
            // filler accounts are a debug only feature. integer division is fine here
            let accounts_per_slot = (count as u64) / slots_per_epoch;
            self.filler_accounts_per_slot
                .store(accounts_per_slot, Ordering::Release);
            self.filler_account_slots_remaining
                .store(slots_per_epoch, Ordering::Release);
        }
    }

    pub fn set_shrink_paths(&self, paths: Vec<PathBuf>) {
        assert!(!paths.is_empty());
        let mut shrink_paths = self.shrink_paths.write().unwrap();
        for path in &paths {
            std::fs::create_dir_all(path).expect("Create directory failed.");
        }
        *shrink_paths = Some(paths);
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn new_single_for_tests() -> Self {
        AccountsDb {
            min_num_stores: 0,
            ..AccountsDb::new_for_tests(Vec::new(), &ClusterType::Development)
        }
    }

    pub fn new_single_for_tests_with_caching() -> Self {
        AccountsDb {
            min_num_stores: 0,
            ..AccountsDb::new_for_tests_with_caching(Vec::new(), &ClusterType::Development)
        }
    }

    fn next_id(&self) -> AppendVecId {
        let next_id = self.next_id.fetch_add(1, Ordering::AcqRel);
        assert!(next_id != AppendVecId::MAX, "We've run out of storage ids!");
        next_id
    }

    fn new_storage_entry(&self, slot: Slot, path: &Path, size: u64) -> AccountStorageEntry {
        AccountStorageEntry::new(path, slot, self.next_id(), size)
    }

    pub fn expected_cluster_type(&self) -> ClusterType {
        self.cluster_type
            .expect("Cluster type must be set at initialization")
    }

    /// Reclaim older states of accounts older than max_clean_root for AccountsDb bloat mitigation
    fn clean_accounts_older_than_root(
        &self,
        purges: Vec<Pubkey>,
        max_clean_root: Option<Slot>,
    ) -> ReclaimResult {
        if purges.is_empty() {
            return ReclaimResult::default();
        }
        // This number isn't carefully chosen; just guessed randomly such that
        // the hot loop will be the order of ~Xms.
        const INDEX_CLEAN_BULK_COUNT: usize = 4096;

        let mut clean_rooted = Measure::start("clean_old_root-ms");
        let reclaim_vecs = purges
            .par_chunks(INDEX_CLEAN_BULK_COUNT)
            .map(|pubkeys: &[Pubkey]| {
                let mut reclaims = Vec::new();
                for pubkey in pubkeys {
                    self.accounts_index
                        .clean_rooted_entries(pubkey, &mut reclaims, max_clean_root);
                }
                reclaims
            });
        let reclaims: Vec<_> = reclaim_vecs.flatten().collect();
        clean_rooted.stop();
        inc_new_counter_info!("clean-old-root-par-clean-ms", clean_rooted.as_ms() as usize);
        self.clean_accounts_stats
            .clean_old_root_us
            .fetch_add(clean_rooted.as_us(), Ordering::Relaxed);

        let mut measure = Measure::start("clean_old_root_reclaims");

        // Don't reset from clean, since the pubkeys in those stores may need to be unref'ed
        // and those stores may be used for background hashing.
        let reset_accounts = false;

        let mut reclaim_result = ReclaimResult::default();
        self.handle_reclaims(
            &reclaims,
            None,
            Some((&self.clean_accounts_stats.purge_stats, &mut reclaim_result)),
            reset_accounts,
        );
        measure.stop();
        debug!("{} {}", clean_rooted, measure);
        inc_new_counter_info!("clean-old-root-reclaim-ms", measure.as_ms() as usize);
        self.clean_accounts_stats
            .clean_old_root_reclaim_us
            .fetch_add(measure.as_us(), Ordering::Relaxed);
        reclaim_result
    }

    fn do_reset_uncleaned_roots(&self, max_clean_root: Option<Slot>) {
        let mut measure = Measure::start("reset");
        self.accounts_index.reset_uncleaned_roots(max_clean_root);
        measure.stop();
        self.clean_accounts_stats
            .reset_uncleaned_roots_us
            .fetch_add(measure.as_us(), Ordering::Relaxed);
    }

    fn calc_delete_dependencies(
        purges: &HashMap<Pubkey, (SlotList<AccountInfo>, RefCount)>,
        store_counts: &mut HashMap<AppendVecId, (usize, HashSet<Pubkey>)>,
    ) {
        // Another pass to check if there are some filtered accounts which
        // do not match the criteria of deleting all appendvecs which contain them
        // then increment their storage count.
        let mut already_counted = HashSet::new();
        for (pubkey, (account_infos, ref_count_from_storage)) in purges.iter() {
            let no_delete = if account_infos.len() as RefCount != *ref_count_from_storage {
                debug!(
                    "calc_delete_dependencies(),
                    pubkey: {},
                    account_infos: {:?},
                    account_infos_len: {},
                    ref_count_from_storage: {}",
                    pubkey,
                    account_infos,
                    account_infos.len(),
                    ref_count_from_storage,
                );
                true
            } else {
                let mut no_delete = false;
                for (_slot, account_info) in account_infos {
                    debug!(
                        "calc_delete_dependencies()
                        storage id: {},
                        count len: {}",
                        account_info.store_id(),
                        store_counts.get(&account_info.store_id()).unwrap().0,
                    );
                    if store_counts.get(&account_info.store_id()).unwrap().0 != 0 {
                        no_delete = true;
                        break;
                    }
                }
                no_delete
            };
            if no_delete {
                let mut pending_store_ids = HashSet::new();
                for (_bank_id, account_info) in account_infos {
                    if !already_counted.contains(&account_info.store_id()) {
                        pending_store_ids.insert(account_info.store_id());
                    }
                }
                while !pending_store_ids.is_empty() {
                    let id = pending_store_ids.iter().next().cloned().unwrap();
                    pending_store_ids.remove(&id);
                    if !already_counted.insert(id) {
                        continue;
                    }
                    store_counts.get_mut(&id).unwrap().0 += 1;

                    let affected_pubkeys = &store_counts.get(&id).unwrap().1;
                    for key in affected_pubkeys {
                        for (_slot, account_info) in &purges.get(key).unwrap().0 {
                            if !already_counted.contains(&account_info.store_id()) {
                                pending_store_ids.insert(account_info.store_id());
                            }
                        }
                    }
                }
            }
        }
    }

    fn background_hasher(receiver: Receiver<CachedAccount>) {
        loop {
            let result = receiver.recv();
            match result {
                Ok(account) => {
                    // if we hold the only ref, then this account doesn't need to be hashed, we ignore this account and it will disappear
                    if Arc::strong_count(&account) > 1 {
                        // this will cause the hash to be calculated and store inside account if it needs to be calculated
                        let _ = (*account).hash();
                    };
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    fn start_background_hasher(&mut self) {
        let (sender, receiver) = unbounded();
        Builder::new()
            .name("solana-db-store-hasher-accounts".to_string())
            .spawn(move || {
                Self::background_hasher(receiver);
            })
            .unwrap();
        self.sender_bg_hasher = Some(sender);
    }

    pub(crate) fn purge_keys_exact<'a, C: 'a>(
        &'a self,
        pubkey_to_slot_set: impl Iterator<Item = &'a (Pubkey, C)>,
    ) -> Vec<(u64, AccountInfo)>
    where
        C: Contains<'a, Slot>,
    {
        let mut reclaims = Vec::new();
        let mut dead_keys = Vec::new();

        for (pubkey, slots_set) in pubkey_to_slot_set {
            let is_empty = self
                .accounts_index
                .purge_exact(pubkey, slots_set, &mut reclaims);
            if is_empty {
                dead_keys.push(pubkey);
            }
        }

        self.accounts_index
            .handle_dead_keys(&dead_keys, &self.account_indexes);
        reclaims
    }

    fn max_clean_root(&self, proposed_clean_root: Option<Slot>) -> Option<Slot> {
        match (
            self.accounts_index.min_ongoing_scan_root(),
            proposed_clean_root,
        ) {
            (None, None) => None,
            (Some(min_scan_root), None) => Some(min_scan_root),
            (None, Some(proposed_clean_root)) => Some(proposed_clean_root),
            (Some(min_scan_root), Some(proposed_clean_root)) => {
                Some(std::cmp::min(min_scan_root, proposed_clean_root))
            }
        }
    }

    /// return 'slot' - slots_in_epoch
    fn get_slot_one_epoch_prior(slot: Slot, epoch_schedule: &EpochSchedule) -> Slot {
        // would like to use:
        // slot.saturating_sub(epoch_schedule.get_slots_in_epoch(epoch_schedule.get_epoch(slot)))
        // but there are problems with warmup and such on tests and probably test clusters.
        // So, just use the maximum below (epoch_schedule.slots_per_epoch)
        slot.saturating_sub(epoch_schedule.slots_per_epoch)
    }

    /// hash calc is completed as of 'slot'
    /// so, any process that wants to take action on really old slots can now proceed up to 'completed_slot'-slots per epoch
    pub fn notify_accounts_hash_calculated_complete(
        &self,
        completed_slot: Slot,
        epoch_schedule: &EpochSchedule,
    ) {
        let one_epoch_old_slot = Self::get_slot_one_epoch_prior(completed_slot, epoch_schedule);
        let mut accounts_hash_complete_one_epoch_old =
            self.accounts_hash_complete_one_epoch_old.write().unwrap();
        *accounts_hash_complete_one_epoch_old =
            std::cmp::max(*accounts_hash_complete_one_epoch_old, one_epoch_old_slot);
        let accounts_hash_complete_one_epoch_old = *accounts_hash_complete_one_epoch_old;

        // now that calculate_accounts_hash_without_index is complete, we can remove old historical roots
        self.remove_old_historical_roots(accounts_hash_complete_one_epoch_old);
    }

    /// get the slot that is one epoch older than the highest slot that has been used for hash calculation
    fn get_accounts_hash_complete_one_epoch_old(&self) -> Slot {
        *self.accounts_hash_complete_one_epoch_old.read().unwrap()
    }

    /// Collect all the uncleaned slots, up to a max slot
    ///
    /// Search through the uncleaned Pubkeys and return all the slots, up to a maximum slot.
    fn collect_uncleaned_slots_up_to_slot(&self, max_slot: Slot) -> Vec<Slot> {
        self.uncleaned_pubkeys
            .iter()
            .filter_map(|entry| {
                let slot = *entry.key();
                (slot <= max_slot).then(|| slot)
            })
            .collect()
    }

    /// Remove `slots` from `uncleaned_pubkeys` and collect all pubkeys
    ///
    /// For each slot in the list of uncleaned slots, remove it from the `uncleaned_pubkeys` Map
    /// and collect all the pubkeys to return.
    fn remove_uncleaned_slots_and_collect_pubkeys(
        &self,
        uncleaned_slots: Vec<Slot>,
    ) -> Vec<Vec<Pubkey>> {
        uncleaned_slots
            .into_iter()
            .filter_map(|uncleaned_slot| {
                self.uncleaned_pubkeys
                    .remove(&uncleaned_slot)
                    .map(|(_removed_slot, removed_pubkeys)| removed_pubkeys)
            })
            .collect()
    }

    /// Remove uncleaned slots, up to a maximum slot, and return the collected pubkeys
    ///
    fn remove_uncleaned_slots_and_collect_pubkeys_up_to_slot(
        &self,
        max_slot: Slot,
    ) -> Vec<Vec<Pubkey>> {
        let uncleaned_slots = self.collect_uncleaned_slots_up_to_slot(max_slot);
        self.remove_uncleaned_slots_and_collect_pubkeys(uncleaned_slots)
    }

    // Construct a vec of pubkeys for cleaning from:
    //   uncleaned_pubkeys - the delta set of updated pubkeys in rooted slots from the last clean
    //   dirty_stores - set of stores which had accounts removed or recently rooted
    fn construct_candidate_clean_keys(
        &self,
        max_clean_root: Option<Slot>,
        last_full_snapshot_slot: Option<Slot>,
        timings: &mut CleanKeyTimings,
    ) -> Vec<Pubkey> {
        let mut dirty_store_processing_time = Measure::start("dirty_store_processing");
        let max_slot = max_clean_root.unwrap_or_else(|| self.accounts_index.max_root_inclusive());
        let mut dirty_stores = Vec::with_capacity(self.dirty_stores.len());
        self.dirty_stores.retain(|(slot, _store_id), store| {
            if *slot > max_slot {
                true
            } else {
                dirty_stores.push((*slot, store.clone()));
                false
            }
        });
        let dirty_stores_len = dirty_stores.len();
        let pubkeys = DashSet::new();
        for (_slot, store) in dirty_stores {
            store.accounts.account_iter().for_each(|account| {
                pubkeys.insert(account.meta.pubkey);
            });
        }
        trace!(
            "dirty_stores.len: {} pubkeys.len: {}",
            dirty_stores_len,
            pubkeys.len()
        );
        timings.dirty_pubkeys_count = pubkeys.len() as u64;
        dirty_store_processing_time.stop();
        timings.dirty_store_processing_us += dirty_store_processing_time.as_us();

        let mut collect_delta_keys = Measure::start("key_create");
        let delta_keys = self.remove_uncleaned_slots_and_collect_pubkeys_up_to_slot(max_slot);
        collect_delta_keys.stop();
        timings.collect_delta_keys_us += collect_delta_keys.as_us();

        let mut delta_insert = Measure::start("delta_insert");
        self.thread_pool_clean.install(|| {
            delta_keys.par_iter().for_each(|keys| {
                for key in keys {
                    pubkeys.insert(*key);
                }
            });
        });
        delta_insert.stop();
        timings.delta_insert_us += delta_insert.as_us();

        timings.delta_key_count = pubkeys.len() as u64;

        let mut hashset_to_vec = Measure::start("flat_map");
        let mut pubkeys: Vec<Pubkey> = pubkeys.into_iter().collect();
        hashset_to_vec.stop();
        timings.hashset_to_vec_us += hashset_to_vec.as_us();

        // Check if we should purge any of the zero_lamport_accounts_to_purge_later, based on the
        // last_full_snapshot_slot.
        assert!(
            last_full_snapshot_slot.is_some() || self.zero_lamport_accounts_to_purge_after_full_snapshot.is_empty(),
            "if snapshots are disabled, then zero_lamport_accounts_to_purge_later should always be empty"
        );
        if let Some(last_full_snapshot_slot) = last_full_snapshot_slot {
            self.zero_lamport_accounts_to_purge_after_full_snapshot
                .retain(|(slot, pubkey)| {
                    let is_candidate_for_clean =
                        max_slot >= *slot && last_full_snapshot_slot >= *slot;
                    if is_candidate_for_clean {
                        pubkeys.push(*pubkey);
                    }
                    !is_candidate_for_clean
                });
        }

        pubkeys
    }

    // Purge zero lamport accounts and older rooted account states as garbage
    // collection
    // Only remove those accounts where the entire rooted history of the account
    // can be purged because there are no live append vecs in the ancestors
    pub fn clean_accounts(
        &self,
        max_clean_root: Option<Slot>,
        is_startup: bool,
        last_full_snapshot_slot: Option<Slot>,
    ) {
        let _guard = self.active_stats.activate(ActiveStatItem::Clean);

        let mut measure_all = Measure::start("clean_accounts");
        let max_clean_root = self.max_clean_root(max_clean_root);

        // hold a lock to prevent slot shrinking from running because it might modify some rooted
        // slot storages which can not happen as long as we're cleaning accounts because we're also
        // modifying the rooted slot storages!
        let mut candidates_v1 = self.shrink_candidate_slots_v1.lock().unwrap();
        self.report_store_stats();

        let mut key_timings = CleanKeyTimings::default();
        let mut pubkeys = self.construct_candidate_clean_keys(
            max_clean_root,
            last_full_snapshot_slot,
            &mut key_timings,
        );

        let mut sort = Measure::start("sort");
        if is_startup {
            pubkeys.par_sort_unstable();
        } else {
            self.thread_pool_clean
                .install(|| pubkeys.par_sort_unstable());
        }
        sort.stop();

        let total_keys_count = pubkeys.len();
        let mut accounts_scan = Measure::start("accounts_scan");
        let uncleaned_roots = self.accounts_index.clone_uncleaned_roots();
        let found_not_zero_accum = AtomicU64::new(0);
        let not_found_on_fork_accum = AtomicU64::new(0);
        let missing_accum = AtomicU64::new(0);
        let useful_accum = AtomicU64::new(0);

        // parallel scan the index.
        let (mut purges_zero_lamports, purges_old_accounts) = {
            let do_clean_scan = || {
                pubkeys
                    .par_chunks(4096)
                    .map(|pubkeys: &[Pubkey]| {
                        let mut purges_zero_lamports = HashMap::new();
                        let mut purges_old_accounts = Vec::new();
                        let mut found_not_zero = 0;
                        let mut not_found_on_fork = 0;
                        let mut missing = 0;
                        let mut useful = 0;
                        self.accounts_index
                            .scan(pubkeys.iter(), |pubkey, slots_refs| {
                                let mut useless = true;
                                if let Some((slot_list, ref_count)) = slots_refs {
                                    let index_in_slot_list = self.accounts_index.latest_slot(
                                        None,
                                        slot_list,
                                        max_clean_root,
                                    );

                                    match index_in_slot_list {
                                        Some(index_in_slot_list) => {
                                            // found info relative to max_clean_root
                                            let (slot, account_info) =
                                                &slot_list[index_in_slot_list];
                                            if account_info.is_zero_lamport() {
                                                useless = false;
                                                purges_zero_lamports.insert(
                                                    *pubkey,
                                                    (
                                                        self.accounts_index.get_rooted_entries(
                                                            slot_list,
                                                            max_clean_root,
                                                        ),
                                                        ref_count,
                                                    ),
                                                );
                                            } else {
                                                found_not_zero += 1;
                                            }
                                            if uncleaned_roots.contains(slot) {
                                                // Assertion enforced by `accounts_index.get()`, the latest slot
                                                // will not be greater than the given `max_clean_root`
                                                if let Some(max_clean_root) = max_clean_root {
                                                    assert!(slot <= &max_clean_root);
                                                }
                                                purges_old_accounts.push(*pubkey);
                                                useless = false;
                                            }
                                        }
                                        None => {
                                            // This pubkey is in the index but not in a root slot, so clean
                                            // it up by adding it to the to-be-purged list.
                                            //
                                            // Also, this pubkey must have been touched by some slot since
                                            // it was in the dirty list, so we assume that the slot it was
                                            // touched in must be unrooted.
                                            not_found_on_fork += 1;
                                            useless = false;
                                            purges_old_accounts.push(*pubkey);
                                        }
                                    }
                                } else {
                                    missing += 1;
                                }
                                if !useless {
                                    useful += 1;
                                }
                                if useless {
                                    AccountsIndexScanResult::None
                                } else {
                                    AccountsIndexScanResult::KeepInMemory
                                }
                            });
                        found_not_zero_accum.fetch_add(found_not_zero, Ordering::Relaxed);
                        not_found_on_fork_accum.fetch_add(not_found_on_fork, Ordering::Relaxed);
                        missing_accum.fetch_add(missing, Ordering::Relaxed);
                        useful_accum.fetch_add(useful, Ordering::Relaxed);
                        (purges_zero_lamports, purges_old_accounts)
                    })
                    .reduce(
                        || (HashMap::new(), Vec::new()),
                        |mut m1, m2| {
                            // Collapse down the hashmaps/vecs into one.
                            m1.0.extend(m2.0);
                            m1.1.extend(m2.1);
                            m1
                        },
                    )
            };
            if is_startup {
                do_clean_scan()
            } else {
                self.thread_pool_clean.install(do_clean_scan)
            }
        };
        accounts_scan.stop();

        let mut clean_old_rooted = Measure::start("clean_old_roots");
        let (purged_account_slots, removed_accounts) =
            self.clean_accounts_older_than_root(purges_old_accounts, max_clean_root);

        if self.caching_enabled {
            self.do_reset_uncleaned_roots(max_clean_root);
        } else {
            self.do_reset_uncleaned_roots_v1(&mut candidates_v1, max_clean_root);
        }
        clean_old_rooted.stop();

        let mut store_counts_time = Measure::start("store_counts");

        // Calculate store counts as if everything was purged
        // Then purge if we can
        let mut store_counts: HashMap<AppendVecId, (usize, HashSet<Pubkey>)> = HashMap::new();
        for (key, (account_infos, ref_count)) in purges_zero_lamports.iter_mut() {
            if purged_account_slots.contains_key(key) {
                *ref_count = self.accounts_index.ref_count_from_storage(key);
            }
            account_infos.retain(|(slot, account_info)| {
                let was_slot_purged = purged_account_slots
                    .get(key)
                    .map(|slots_removed| slots_removed.contains(slot))
                    .unwrap_or(false);
                if was_slot_purged {
                    // No need to look up the slot storage below if the entire
                    // slot was purged
                    return false;
                }
                // Check if this update in `slot` to the account with `key` was reclaimed earlier by
                // `clean_accounts_older_than_root()`
                let was_reclaimed = removed_accounts
                    .get(&account_info.store_id())
                    .map(|store_removed| store_removed.contains(&account_info.offset()))
                    .unwrap_or(false);
                if was_reclaimed {
                    return false;
                }
                if let Some(store_count) = store_counts.get_mut(&account_info.store_id()) {
                    store_count.0 -= 1;
                    store_count.1.insert(*key);
                } else {
                    let mut key_set = HashSet::new();
                    key_set.insert(*key);
                    assert!(
                        !account_info.is_cached(),
                        "The Accounts Cache must be flushed first for this account info. pubkey: {}, slot: {}",
                        *key,
                        *slot
                    );
                    let count = self
                        .storage
                        .slot_store_count(*slot, account_info.store_id())
                        .unwrap()
                        - 1;
                    debug!(
                        "store_counts, inserting slot: {}, store id: {}, count: {}",
                        slot, account_info.store_id(), count
                    );
                    store_counts.insert(account_info.store_id(), (count, key_set));
                }
                true
            });
        }
        store_counts_time.stop();

        let mut calc_deps_time = Measure::start("calc_deps");
        Self::calc_delete_dependencies(&purges_zero_lamports, &mut store_counts);
        calc_deps_time.stop();

        let mut purge_filter = Measure::start("purge_filter");
        self.filter_zero_lamport_clean_for_incremental_snapshots(
            max_clean_root,
            last_full_snapshot_slot,
            &store_counts,
            &mut purges_zero_lamports,
        );
        purge_filter.stop();

        let mut reclaims_time = Measure::start("reclaims");
        // Recalculate reclaims with new purge set
        let pubkey_to_slot_set: Vec<_> = purges_zero_lamports
            .into_iter()
            .map(|(key, (slots_list, _ref_count))| {
                (
                    key,
                    slots_list
                        .into_iter()
                        .map(|(slot, _)| slot)
                        .collect::<HashSet<Slot>>(),
                )
            })
            .collect();

        let reclaims = self.purge_keys_exact(pubkey_to_slot_set.iter());

        // Don't reset from clean, since the pubkeys in those stores may need to be unref'ed
        // and those stores may be used for background hashing.
        let reset_accounts = false;
        let mut reclaim_result = ReclaimResult::default();
        self.handle_reclaims(
            &reclaims,
            None,
            Some((&self.clean_accounts_stats.purge_stats, &mut reclaim_result)),
            reset_accounts,
        );

        reclaims_time.stop();
        measure_all.stop();

        self.clean_accounts_stats.report();
        datapoint_info!(
            "clean_accounts",
            ("total_us", measure_all.as_us(), i64),
            (
                "collect_delta_keys_us",
                key_timings.collect_delta_keys_us,
                i64
            ),
            (
                "dirty_store_processing_us",
                key_timings.dirty_store_processing_us,
                i64
            ),
            ("accounts_scan", accounts_scan.as_us() as i64, i64),
            ("clean_old_rooted", clean_old_rooted.as_us() as i64, i64),
            ("store_counts", store_counts_time.as_us() as i64, i64),
            ("purge_filter", purge_filter.as_us() as i64, i64),
            ("calc_deps", calc_deps_time.as_us() as i64, i64),
            ("reclaims", reclaims_time.as_us() as i64, i64),
            ("delta_insert_us", key_timings.delta_insert_us, i64),
            ("delta_key_count", key_timings.delta_key_count, i64),
            ("dirty_pubkeys_count", key_timings.dirty_pubkeys_count, i64),
            ("sort_us", sort.as_us(), i64),
            ("useful_keys", useful_accum.load(Ordering::Relaxed), i64),
            ("total_keys_count", total_keys_count, i64),
            (
                "scan_found_not_zero",
                found_not_zero_accum.load(Ordering::Relaxed),
                i64
            ),
            (
                "scan_not_found_on_fork",
                not_found_on_fork_accum.load(Ordering::Relaxed),
                i64
            ),
            ("scan_missing", missing_accum.load(Ordering::Relaxed), i64),
            ("uncleaned_roots_len", uncleaned_roots.len(), i64),
            (
                "clean_old_root_us",
                self.clean_accounts_stats
                    .clean_old_root_us
                    .swap(0, Ordering::Relaxed),
                i64
            ),
            (
                "clean_old_root_reclaim_us",
                self.clean_accounts_stats
                    .clean_old_root_reclaim_us
                    .swap(0, Ordering::Relaxed),
                i64
            ),
            (
                "reset_uncleaned_roots_us",
                self.clean_accounts_stats
                    .reset_uncleaned_roots_us
                    .swap(0, Ordering::Relaxed),
                i64
            ),
            (
                "remove_dead_accounts_remove_us",
                self.clean_accounts_stats
                    .remove_dead_accounts_remove_us
                    .swap(0, Ordering::Relaxed),
                i64
            ),
            (
                "remove_dead_accounts_shrink_us",
                self.clean_accounts_stats
                    .remove_dead_accounts_shrink_us
                    .swap(0, Ordering::Relaxed),
                i64
            ),
            (
                "clean_stored_dead_slots_us",
                self.clean_accounts_stats
                    .clean_stored_dead_slots_us
                    .swap(0, Ordering::Relaxed),
                i64
            ),
            (
                "roots_added",
                self.accounts_index.roots_added.swap(0, Ordering::Relaxed) as i64,
                i64
            ),
            (
                "roots_removed",
                self.accounts_index.roots_removed.swap(0, Ordering::Relaxed) as i64,
                i64
            ),
            (
                "active_scans",
                self.accounts_index.active_scans.load(Ordering::Relaxed) as i64,
                i64
            ),
            (
                "max_distance_to_min_scan_slot",
                self.accounts_index
                    .max_distance_to_min_scan_slot
                    .swap(0, Ordering::Relaxed),
                i64
            ),
            ("next_store_id", self.next_id.load(Ordering::Relaxed), i64),
        );
    }

    /// Removes the accounts in the input `reclaims` from the tracked "count" of
    /// their corresponding  storage entries. Note this does not actually free
    /// the memory from the storage entries until all the storage entries for
    /// a given slot `S` are empty, at which point `process_dead_slots` will
    /// remove all the storage entries for `S`.
    ///
    /// # Arguments
    /// * `reclaims` - The accounts to remove from storage entries' "count". Note here
    ///    that we should not remove cache entries, only entries for accounts actually
    ///    stored in a storage entry.
    ///
    /// * `expected_single_dead_slot` - A correctness assertion. If this is equal to `Some(S)`,
    ///    then the function will check that the only slot being cleaned up in `reclaims`
    ///    is the slot == `S`. This is true for instance when `handle_reclaims` is called
    ///    from store or slot shrinking, as those should only touch the slot they are
    ///    currently storing to or shrinking.
    ///
    /// * `purge_stats_and_reclaim_result` - Option containing `purge_stats` and `reclaim_result`.
    ///    `purge_stats`. `purge_stats` are stats used to track performance of purging dead slots.
    ///    `reclaim_result` contains information about accounts that were removed from storage,
    ///    does not include accounts that were removed from the cache.
    ///    If `purge_stats_and_reclaim_result.is_none()`, this implies there can be no dead slots
    ///    that happen as a result of this call, and the function will check that no slots are
    ///    cleaned up/removed via `process_dead_slots`. For instance, on store, no slots should
    ///    be cleaned up, but during the background clean accounts purges accounts from old rooted
    ///    slots, so outdated slots may be removed.
    ///
    /// * `reset_accounts` - Reset the append_vec store when the store is dead (count==0)
    ///    From the clean and shrink paths it should be false since there may be an in-progress
    ///    hash operation and the stores may hold accounts that need to be unref'ed.
    fn handle_reclaims(
        &self,
        reclaims: SlotSlice<AccountInfo>,
        expected_single_dead_slot: Option<Slot>,
        purge_stats_and_reclaim_result: Option<(&PurgeStats, &mut ReclaimResult)>,
        reset_accounts: bool,
    ) {
        if reclaims.is_empty() {
            return;
        }

        let (purge_stats, purged_account_slots, reclaimed_offsets) =
            if let Some((purge_stats, (ref mut purged_account_slots, ref mut reclaimed_offsets))) =
                purge_stats_and_reclaim_result
            {
                (
                    Some(purge_stats),
                    Some(purged_account_slots),
                    Some(reclaimed_offsets),
                )
            } else {
                (None, None, None)
            };

        let dead_slots = self.remove_dead_accounts(
            reclaims,
            expected_single_dead_slot,
            reclaimed_offsets,
            reset_accounts,
        );

        if let Some(purge_stats) = purge_stats {
            if let Some(expected_single_dead_slot) = expected_single_dead_slot {
                assert!(dead_slots.len() <= 1);
                if dead_slots.len() == 1 {
                    assert!(dead_slots.contains(&expected_single_dead_slot));
                }
            }

            self.process_dead_slots(&dead_slots, purged_account_slots, purge_stats);
        } else {
            // not sure why this fails yet with ancient append vecs
            if !self.ancient_append_vecs {
                assert!(dead_slots.is_empty());
            }
        }
    }

    /// During clean, some zero-lamport accounts that are marked for purge should *not* actually
    /// get purged.  Filter out those accounts here.
    ///
    /// When using incremental snapshots, do not purge zero-lamport accounts if the slot is higher
    /// than the last full snapshot slot.  This is to protect against the following scenario:
    ///
    ///   ```text
    ///   A full snapshot is taken, and it contains an account with a non-zero balance.  Later,
    ///   that account's  goes to zero.  Evntually cleaning runs, and before, this account would be
    ///   cleaned up.  Finally, an incremental snapshot is taken.
    ///
    ///   Later, the incremental (and full) snapshot is used to rebuild the bank and accounts
    ///   database (e.x. if the node restarts).  The full snapshot _does_ contain the account (from
    ///   above) and its balance is non-zero, however, since the account was cleaned up in a later
    ///   slot, the incremental snapshot does not contain any info about this account, thus, the
    ///   accounts database will contain the old info from this account, which has its old non-zero
    ///   balance.  Very bad!
    ///   ```
    ///
    /// This filtering step can be skipped if there is no `last_full_snapshot_slot`, or if the
    /// `max_clean_root` is less-than-or-equal-to the `last_full_snapshot_slot`.
    fn filter_zero_lamport_clean_for_incremental_snapshots(
        &self,
        max_clean_root: Option<Slot>,
        last_full_snapshot_slot: Option<Slot>,
        store_counts: &HashMap<AppendVecId, (usize, HashSet<Pubkey>)>,
        purges_zero_lamports: &mut HashMap<Pubkey, (SlotList<AccountInfo>, RefCount)>,
    ) {
        let should_filter_for_incremental_snapshots =
            max_clean_root.unwrap_or(Slot::MAX) > last_full_snapshot_slot.unwrap_or(Slot::MAX);
        assert!(
            last_full_snapshot_slot.is_some() || !should_filter_for_incremental_snapshots,
            "if filtering for incremental snapshots, then snapshots should be enabled",
        );

        purges_zero_lamports.retain(|pubkey, (slot_account_infos, _ref_count)| {
            // Only keep purges_zero_lamports where the entire history of the account in the root set
            // can be purged. All AppendVecs for those updates are dead.
            for (_slot, account_info) in slot_account_infos.iter() {
                if store_counts.get(&account_info.store_id()).unwrap().0 != 0 {
                    return false;
                }
            }

            // Exit early if not filtering more for incremental snapshots
            if !should_filter_for_incremental_snapshots {
                return true;
            }

            let slot_account_info_at_highest_slot = slot_account_infos
                .iter()
                .max_by_key(|(slot, _account_info)| slot);

            slot_account_info_at_highest_slot.map_or(true, |(slot, account_info)| {
                // Do *not* purge zero-lamport accounts if the slot is greater than the last full
                // snapshot slot.  Since we're `retain`ing the accounts-to-purge, I felt creating
                // the `cannot_purge` variable made this easier to understand.  Accounts that do
                // not get purged here are added to a list so they be considered for purging later
                // (i.e. after the next full snapshot).
                assert!(account_info.is_zero_lamport());
                let cannot_purge = *slot > last_full_snapshot_slot.unwrap();
                if cannot_purge {
                    self.zero_lamport_accounts_to_purge_after_full_snapshot
                        .insert((*slot, *pubkey));
                }
                !cannot_purge
            })
        });
    }

    // Must be kept private!, does sensitive cleanup that should only be called from
    // supported pipelines in AccountsDb
    fn process_dead_slots(
        &self,
        dead_slots: &HashSet<Slot>,
        purged_account_slots: Option<&mut AccountSlots>,
        purge_stats: &PurgeStats,
    ) {
        if dead_slots.is_empty() {
            return;
        }
        let mut clean_dead_slots = Measure::start("reclaims::clean_dead_slots");
        self.clean_stored_dead_slots(dead_slots, purged_account_slots);
        clean_dead_slots.stop();

        let mut purge_removed_slots = Measure::start("reclaims::purge_removed_slots");
        self.purge_dead_slots_from_storage(dead_slots.iter(), purge_stats);
        purge_removed_slots.stop();

        // If the slot is dead, remove the need to shrink the storages as
        // the storage entries will be purged.
        {
            let mut list = self.shrink_candidate_slots.lock().unwrap();
            for slot in dead_slots {
                list.remove(slot);
            }
        }

        debug!(
            "process_dead_slots({}): {} {} {:?}",
            dead_slots.len(),
            clean_dead_slots,
            purge_removed_slots,
            dead_slots,
        );
    }

    /// load the account index entry for the first `count` items in `accounts`
    /// store a reference to all alive accounts in `alive_accounts`
    /// unref and optionally store a reference to all pubkeys that are in the index, but dead in `unrefed_pubkeys`
    /// return sum of account size for all alive accounts
    fn load_accounts_index_for_shrink<'a>(
        &'a self,
        accounts: &'a [(Pubkey, FoundStoredAccount<'a>)],
        count: usize,
        alive_accounts: &mut Vec<&'a (Pubkey, FoundStoredAccount<'a>)>,
        mut unrefed_pubkeys: Option<&mut Vec<&'a Pubkey>>,
    ) -> usize {
        let mut alive_total = 0;

        let mut alive = 0;
        let mut dead = 0;
        let mut index = 0;
        self.accounts_index.scan(
            accounts[..std::cmp::min(accounts.len(), count)]
                .iter()
                .map(|(key, _)| key),
            |pubkey, slots_refs| {
                let mut result = AccountsIndexScanResult::None;
                if let Some((slot_list, _ref_count)) = slots_refs {
                    let pair = &accounts[index];
                    let stored_account = &pair.1;
                    let is_alive = slot_list.iter().any(|(_slot, acct_info)| {
                        acct_info.matches_storage_location(
                            stored_account.store_id,
                            stored_account.account.offset,
                        )
                    });
                    if !is_alive {
                        // This pubkey was found in the storage, but no longer exists in the index.
                        // It would have had a ref to the storage from the initial store, but it will
                        // not exist in the re-written slot. Unref it to keep the index consistent with
                        // rewriting the storage entries.
                        if let Some(unrefed_pubkeys) = &mut unrefed_pubkeys {
                            unrefed_pubkeys.push(pubkey);
                        }
                        result = AccountsIndexScanResult::Unref;
                        dead += 1;
                    } else {
                        alive_accounts.push(pair);
                        alive_total += stored_account.account.stored_size;
                        alive += 1;
                    }
                }
                index += 1;
                result
            },
        );
        assert_eq!(index, std::cmp::min(accounts.len(), count));
        self.shrink_stats
            .alive_accounts
            .fetch_add(alive, Ordering::Relaxed);
        self.shrink_stats
            .dead_accounts
            .fetch_add(dead, Ordering::Relaxed);

        alive_total
    }

    /// get all accounts in all the storages passed in
    /// for duplicate pubkeys, the account with the highest write_value is returned
    pub(crate) fn get_unique_accounts_from_storages<'a, I>(
        &'a self,
        stores: I,
    ) -> GetUniqueAccountsResult<'a>
    where
        I: Iterator<Item = &'a Arc<AccountStorageEntry>>,
    {
        let mut stored_accounts: HashMap<Pubkey, FoundStoredAccount> = HashMap::new();
        let mut original_bytes = 0;
        let store_ids = stores
            .into_iter()
            .map(|store| {
                original_bytes += store.total_bytes();
                let store_id = store.append_vec_id();
                store.accounts.account_iter().for_each(|account| {
                    let new_entry = FoundStoredAccount { account, store_id };
                    match stored_accounts.entry(new_entry.account.meta.pubkey) {
                        Entry::Occupied(mut occupied_entry) => {
                            if new_entry.account.meta.write_version
                                > occupied_entry.get().account.meta.write_version
                            {
                                occupied_entry.insert(new_entry);
                            }
                        }
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(new_entry);
                        }
                    }
                });
                store.append_vec_id()
            })
            .collect();
        GetUniqueAccountsResult {
            stored_accounts,
            original_bytes,
            store_ids,
        }
    }

    fn do_shrink_slot_stores<'a, I>(&'a self, slot: Slot, stores: I) -> usize
    where
        I: Iterator<Item = &'a Arc<AccountStorageEntry>>,
    {
        debug!("do_shrink_slot_stores: slot: {}", slot);
        let GetUniqueAccountsResult {
            stored_accounts,
            original_bytes,
            store_ids,
        } = self.get_unique_accounts_from_storages(stores);

        // sort by pubkey to keep account index lookups close
        let mut stored_accounts = stored_accounts.into_iter().collect::<Vec<_>>();
        stored_accounts.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        let mut index_read_elapsed = Measure::start("index_read_elapsed");
        let alive_total_collect = AtomicUsize::new(0);

        let len = stored_accounts.len();
        let alive_accounts_collect = Mutex::new(Vec::with_capacity(len));
        let unrefed_pubkeys_collect = Mutex::new(Vec::with_capacity(len));
        self.shrink_stats
            .accounts_loaded
            .fetch_add(len as u64, Ordering::Relaxed);

        self.thread_pool_clean.install(|| {
            let chunk_size = 50; // # accounts/thread
            let chunks = len / chunk_size + 1;
            (0..chunks).into_par_iter().for_each(|chunk| {
                let skip = chunk * chunk_size;

                let mut alive_accounts = Vec::with_capacity(chunk_size);
                let mut unrefed_pubkeys = Vec::with_capacity(chunk_size);
                let alive_total = self.load_accounts_index_for_shrink(
                    &stored_accounts[skip..],
                    chunk_size,
                    &mut alive_accounts,
                    Some(&mut unrefed_pubkeys),
                );

                // collect
                alive_accounts_collect
                    .lock()
                    .unwrap()
                    .append(&mut alive_accounts);
                unrefed_pubkeys_collect
                    .lock()
                    .unwrap()
                    .append(&mut unrefed_pubkeys);
                alive_total_collect.fetch_add(alive_total, Ordering::Relaxed);
            });
        });

        let alive_accounts = alive_accounts_collect.into_inner().unwrap();
        let unrefed_pubkeys = unrefed_pubkeys_collect.into_inner().unwrap();
        let alive_total = alive_total_collect.load(Ordering::Relaxed);

        index_read_elapsed.stop();
        let aligned_total: u64 = Self::page_align(alive_total as u64);

        // This shouldn't happen if alive_bytes/approx_stored_count are accurate
        if Self::should_not_shrink(aligned_total, original_bytes, store_ids.len()) {
            self.shrink_stats
                .skipped_shrink
                .fetch_add(1, Ordering::Relaxed);
            for pubkey in unrefed_pubkeys {
                if let Some(locked_entry) = self.accounts_index.get_account_read_entry(pubkey) {
                    locked_entry.addref();
                }
            }
            return 0;
        }

        let total_starting_accounts = stored_accounts.len();
        let total_accounts_after_shrink = alive_accounts.len();
        debug!(
            "shrinking: slot: {}, accounts: ({} => {}) bytes: ({} ; aligned to: {}) original: {}",
            slot,
            total_starting_accounts,
            total_accounts_after_shrink,
            alive_total,
            aligned_total,
            original_bytes,
        );

        let mut rewrite_elapsed = Measure::start("rewrite_elapsed");
        let mut dead_storages = vec![];
        let mut find_alive_elapsed = 0;
        let mut create_and_insert_store_elapsed = 0;
        let mut write_storage_elapsed = 0;
        let mut store_accounts_timing = StoreAccountsTiming::default();
        if aligned_total > 0 {
            let mut start = Measure::start("find_alive_elapsed");
            let mut accounts = Vec::with_capacity(total_accounts_after_shrink);
            let mut hashes = Vec::with_capacity(total_accounts_after_shrink);
            let mut write_versions = Vec::with_capacity(total_accounts_after_shrink);

            for (pubkey, alive_account) in alive_accounts {
                accounts.push((pubkey, &alive_account.account));
                hashes.push(alive_account.account.hash);
                write_versions.push(alive_account.account.meta.write_version);
            }
            start.stop();
            find_alive_elapsed = start.as_us();

            let (shrunken_store, time) = self.get_store_for_shrink(slot, aligned_total);
            create_and_insert_store_elapsed = time.as_micros() as u64;

            // here, we're writing back alive_accounts. That should be an atomic operation
            // without use of rather wide locks in this whole function, because we're
            // mutating rooted slots; There should be no writers to them.
            store_accounts_timing = self.store_accounts_frozen(
                (slot, &accounts[..]),
                Some(&hashes),
                Some(&shrunken_store),
                Some(Box::new(write_versions.into_iter())),
                StoreReclaims::Ignore,
            );

            // `store_accounts_frozen()` above may have purged accounts from some
            // other storage entries (the ones that were just overwritten by this
            // new storage entry). This means some of those stores might have caused
            // this slot to be read to `self.shrink_candidate_slots`, so delete
            // those here
            self.shrink_candidate_slots.lock().unwrap().remove(&slot);

            // Purge old, overwritten storage entries
            let mut start = Measure::start("write_storage_elapsed");
            self.mark_dirty_dead_stores(slot, &mut dead_storages, |store| {
                !store_ids.contains(&store.append_vec_id())
            });
            start.stop();
            write_storage_elapsed = start.as_us();
        }
        rewrite_elapsed.stop();

        self.drop_or_recycle_stores(dead_storages);

        self.shrink_stats
            .num_slots_shrunk
            .fetch_add(1, Ordering::Relaxed);
        self.shrink_stats
            .index_read_elapsed
            .fetch_add(index_read_elapsed.as_us(), Ordering::Relaxed);
        self.shrink_stats
            .find_alive_elapsed
            .fetch_add(find_alive_elapsed, Ordering::Relaxed);
        self.shrink_stats
            .create_and_insert_store_elapsed
            .fetch_add(create_and_insert_store_elapsed, Ordering::Relaxed);
        self.shrink_stats.store_accounts_elapsed.fetch_add(
            store_accounts_timing.store_accounts_elapsed,
            Ordering::Relaxed,
        );
        self.shrink_stats.update_index_elapsed.fetch_add(
            store_accounts_timing.update_index_elapsed,
            Ordering::Relaxed,
        );
        self.shrink_stats.handle_reclaims_elapsed.fetch_add(
            store_accounts_timing.handle_reclaims_elapsed,
            Ordering::Relaxed,
        );
        self.shrink_stats
            .write_storage_elapsed
            .fetch_add(write_storage_elapsed, Ordering::Relaxed);
        self.shrink_stats
            .rewrite_elapsed
            .fetch_add(rewrite_elapsed.as_us(), Ordering::Relaxed);
        self.shrink_stats.accounts_removed.fetch_add(
            total_starting_accounts - total_accounts_after_shrink,
            Ordering::Relaxed,
        );
        self.shrink_stats.bytes_removed.fetch_add(
            original_bytes.saturating_sub(aligned_total),
            Ordering::Relaxed,
        );
        self.shrink_stats
            .bytes_written
            .fetch_add(aligned_total, Ordering::Relaxed);

        self.shrink_stats.report();

        total_accounts_after_shrink
    }

    /// get stores for 'slot'
    /// retain only the stores where 'should_retain(store)' == true
    /// for stores not retained, insert in 'dirty_stores' and 'dead_storages'
    pub(crate) fn mark_dirty_dead_stores(
        &self,
        slot: Slot,
        dead_storages: &mut Vec<Arc<AccountStorageEntry>>,
        should_retain: impl Fn(&AccountStorageEntry) -> bool,
    ) {
        if let Some(slot_stores) = self.storage.get_slot_stores(slot) {
            slot_stores.write().unwrap().retain(|_key, store| {
                if !should_retain(store) {
                    self.dirty_stores
                        .insert((slot, store.append_vec_id()), store.clone());
                    dead_storages.push(store.clone());
                    false
                } else {
                    true
                }
            });
        }
    }

    pub(crate) fn drop_or_recycle_stores(&self, dead_storages: Vec<Arc<AccountStorageEntry>>) {
        let mut recycle_stores_write_elapsed = Measure::start("recycle_stores_write_time");
        let mut recycle_stores = self.recycle_stores.write().unwrap();
        recycle_stores_write_elapsed.stop();

        let mut drop_storage_entries_elapsed = Measure::start("drop_storage_entries_elapsed");
        if recycle_stores.entry_count() < MAX_RECYCLE_STORES {
            recycle_stores.add_entries(dead_storages);
            drop(recycle_stores);
        } else {
            self.stats
                .dropped_stores
                .fetch_add(dead_storages.len() as u64, Ordering::Relaxed);
            drop(recycle_stores);
            drop(dead_storages);
        }
        drop_storage_entries_elapsed.stop();
        self.shrink_stats
            .drop_storage_entries_elapsed
            .fetch_add(drop_storage_entries_elapsed.as_us(), Ordering::Relaxed);
        self.shrink_stats
            .recycle_stores_write_elapsed
            .fetch_add(recycle_stores_write_elapsed.as_us(), Ordering::Relaxed);
    }

    /// return a store that can contain 'aligned_total' bytes and the time it took to execute
    pub(crate) fn get_store_for_shrink(
        &self,
        slot: Slot,
        aligned_total: u64,
    ) -> (Arc<AccountStorageEntry>, Duration) {
        let mut start = Measure::start("create_and_insert_store_elapsed");
        let shrunken_store = if let Some(new_store) =
            self.try_recycle_and_insert_store(slot, aligned_total, aligned_total + 1024)
        {
            new_store
        } else {
            let maybe_shrink_paths = self.shrink_paths.read().unwrap();
            if let Some(ref shrink_paths) = *maybe_shrink_paths {
                self.create_and_insert_store_with_paths(
                    slot,
                    aligned_total,
                    "shrink-w-path",
                    shrink_paths,
                )
            } else {
                self.create_and_insert_store(slot, aligned_total, "shrink")
            }
        };
        start.stop();
        (shrunken_store, Duration::from_micros(start.as_us()))
    }

    // Reads all accounts in given slot's AppendVecs and filter only to alive,
    // then create a minimum AppendVec filled with the alive.
    fn shrink_slot_forced(&self, slot: Slot) -> usize {
        debug!("shrink_slot_forced: slot: {}", slot);

        if let Some(stores_lock) = self.storage.get_slot_stores(slot) {
            let stores: Vec<Arc<AccountStorageEntry>> =
                stores_lock.read().unwrap().values().cloned().collect();
            if !Self::is_shrinking_productive(slot, stores.iter()) {
                return 0;
            }
            self.do_shrink_slot_stores(slot, stores.iter())
        } else {
            0
        }
    }

    fn all_slots_in_storage(&self) -> Vec<Slot> {
        self.storage.all_slots()
    }

    fn all_alive_roots_in_index(&self) -> Vec<Slot> {
        self.accounts_index.all_alive_roots()
    }

    /// Given the input `ShrinkCandidates`, this function sorts the stores by their alive ratio
    /// in increasing order with the most sparse entries in the front. It will then simulate the
    /// shrinking by working on the most sparse entries first and if the overall alive ratio is
    /// achieved, it will stop and return the filtered-down candidates and the candidates which
    /// are skipped in this round and might be eligible for the future shrink.
    fn select_candidates_by_total_usage(
        shrink_slots: &ShrinkCandidates,
        shrink_ratio: f64,
    ) -> (ShrinkCandidates, ShrinkCandidates) {
        struct StoreUsageInfo {
            slot: Slot,
            alive_ratio: f64,
            store: Arc<AccountStorageEntry>,
        }
        let mut measure = Measure::start("select_top_sparse_storage_entries-ms");
        let mut store_usage: Vec<StoreUsageInfo> = Vec::with_capacity(shrink_slots.len());
        let mut total_alive_bytes: u64 = 0;
        let mut candidates_count: usize = 0;
        let mut total_bytes: u64 = 0;
        let mut total_candidate_stores: usize = 0;
        for (slot, slot_shrink_candidates) in shrink_slots {
            candidates_count += slot_shrink_candidates.len();
            for store in slot_shrink_candidates.values() {
                total_alive_bytes += Self::page_align(store.alive_bytes() as u64);
                total_bytes += store.total_bytes();
                let alive_ratio = Self::page_align(store.alive_bytes() as u64) as f64
                    / store.total_bytes() as f64;
                store_usage.push(StoreUsageInfo {
                    slot: *slot,
                    alive_ratio,
                    store: store.clone(),
                });
                total_candidate_stores += 1;
            }
        }
        store_usage.sort_by(|a, b| {
            a.alive_ratio
                .partial_cmp(&b.alive_ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Working from the beginning of store_usage which are the most sparse and see when we can stop
        // shrinking while still achieving the overall goals.
        let mut shrink_slots: ShrinkCandidates = HashMap::new();
        let mut shrink_slots_next_batch: ShrinkCandidates = HashMap::new();
        for usage in &store_usage {
            let store = &usage.store;
            let alive_ratio = (total_alive_bytes as f64) / (total_bytes as f64);
            debug!("alive_ratio: {:?} store_id: {:?}, store_ratio: {:?} requirment: {:?}, total_bytes: {:?} total_alive_bytes: {:?}",
                alive_ratio, usage.store.append_vec_id(), usage.alive_ratio, shrink_ratio, total_bytes, total_alive_bytes);
            if alive_ratio > shrink_ratio {
                // we have reached our goal, stop
                debug!(
                    "Shrinking goal can be achieved at slot {:?}, total_alive_bytes: {:?} \
                    total_bytes: {:?}, alive_ratio: {:}, shrink_ratio: {:?}",
                    usage.slot, total_alive_bytes, total_bytes, alive_ratio, shrink_ratio
                );
                if usage.alive_ratio < shrink_ratio {
                    shrink_slots_next_batch
                        .entry(usage.slot)
                        .or_default()
                        .insert(store.append_vec_id(), store.clone());
                } else {
                    break;
                }
            } else {
                let current_store_size = store.total_bytes();
                let after_shrink_size = Self::page_align(store.alive_bytes() as u64);
                let bytes_saved = current_store_size.saturating_sub(after_shrink_size);
                total_bytes -= bytes_saved;
                shrink_slots
                    .entry(usage.slot)
                    .or_default()
                    .insert(store.append_vec_id(), store.clone());
            }
        }
        measure.stop();
        inc_new_counter_debug!(
            "shrink_select_top_sparse_storage_entries-ms",
            measure.as_ms() as usize
        );
        inc_new_counter_debug!(
            "shrink_select_top_sparse_storage_entries-seeds",
            candidates_count
        );
        inc_new_counter_debug!(
            "shrink_total_preliminary_candidate_stores",
            total_candidate_stores
        );

        (shrink_slots, shrink_slots_next_batch)
    }

    fn get_roots_less_than(&self, slot: Slot) -> Vec<Slot> {
        self.accounts_index
            .roots_tracker
            .read()
            .unwrap()
            .alive_roots
            .get_all_less_than(slot)
    }

    fn get_prior_root(&self, slot: Slot) -> Option<Slot> {
        self.accounts_index
            .roots_tracker
            .read()
            .unwrap()
            .alive_roots
            .get_prior(slot)
    }

    /// get a sorted list of slots older than an epoch
    /// squash those slots into ancient append vecs
    fn shrink_ancient_slots(&self) {
        if !self.ancient_append_vecs {
            return;
        }

        // If we squash accounts in a slot that is still within an epoch of a hash calculation's max slot, then
        //  we could calculate the wrong rent_epoch and slot for an individual account and thus the wrong overall accounts hash.
        // So, only squash accounts in slots that are more than 1 epoch older than the last hash calculation.
        // Subsequent hash calculations should be a higher slot.
        let mut old_slots =
            self.get_roots_less_than(self.get_accounts_hash_complete_one_epoch_old());
        old_slots.sort_unstable();
        self.combine_ancient_slots(old_slots);
    }

    /// create new ancient append vec
    /// return it and the elapsed time for metrics
    fn create_ancient_append_vec(
        &self,
        slot: Slot,
    ) -> (Option<(Slot, Arc<AccountStorageEntry>)>, Duration) {
        let (new_ancient_storage, time) =
            self.get_store_for_shrink(slot, get_ancient_append_vec_capacity());
        info!(
            "ancient_append_vec: creating initial ancient append vec: {}, size: {}, id: {}",
            slot,
            get_ancient_append_vec_capacity(),
            new_ancient_storage.append_vec_id(),
        );
        (Some((slot, new_ancient_storage)), time)
    }

    /// return true if created
    /// also return elapsed time for metrics
    fn maybe_create_ancient_append_vec(
        &self,
        current_ancient: &mut Option<(Slot, Arc<AccountStorageEntry>)>,
        slot: Slot,
    ) -> (bool, Duration) {
        if current_ancient.is_none() {
            // our oldest slot is not an append vec of max size, or we filled the previous one.
            // So, create a new ancient append vec at 'slot'
            let result = self.create_ancient_append_vec(slot);
            *current_ancient = result.0;
            (true, result.1)
        } else {
            (false, Duration::default())
        }
    }

    fn get_storages_for_slot(&self, slot: Slot) -> Option<SnapshotStorage> {
        self.storage.map.get(&slot).map(|storages| {
            // per slot, get the storages. There should usually only be 1.
            storages
                .value()
                .read()
                .unwrap()
                .values()
                .cloned()
                .collect::<Vec<_>>()
        })
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
                .then(|| all_storages)
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
        if is_full_ancient(accounts) {
            if self.is_candidate_for_shrink(storage, true) {
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
            if current_ancient.is_some() {
                info!("ancient_append_vec: shrinking full ancient: {}", slot);
            }

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

        if let Some(first_slot) = sorted_slots.first() {
            info!(
                "ancient_append_vec: combine_ancient_slots first slot: {}, num_roots: {}",
                first_slot,
                sorted_slots.len()
            );
        }

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
                guard = Some(self.active_stats.activate(ActiveStatItem::SquashAncient))
            }

            // this code is copied from shrink. I would like to combine it into a helper function, but the borrow checker has defeated my efforts so far.
            let GetUniqueAccountsResult {
                stored_accounts,
                original_bytes,
                store_ids: _,
            } = self.get_unique_accounts_from_storages(old_storages.iter());

            // sort by pubkey to keep account index lookups close
            let mut stored_accounts = stored_accounts.into_iter().collect::<Vec<_>>();
            stored_accounts.sort_unstable_by(|a, b| a.0.cmp(&b.0));

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
            // we could sort these
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

        // Failsafe for potential race conditions with other subsystems
        let mut num_acceptable_failed_iterations = 0;
        loop {
            let account_accessor = self.get_account_accessor(slot, pubkey, &storage_location);
            match account_accessor {
                LoadedAccountAccessor::Cached(Some(_)) | LoadedAccountAccessor::Stored(Some(_)) => {
                    // Great! There was no race, just return :) This is the most usual situation
                    return Some((account_accessor, slot));
                }
                LoadedAccountAccessor::Cached(None) => {
                    num_acceptable_failed_iterations += 1;
                    // Cache was flushed in between checking the index and retrieving from the cache,
                    // so retry. This works because in accounts cache flush, an account is written to
                    // storage *before* it is removed from the cache
                    match load_hint {
                        LoadHint::FixedMaxRoot => {
                            // it's impossible for this to fail for transaction loads from
                            // replaying/banking more than once.
                            // This is because:
                            // 1) For a slot `X` that's being replayed, there is only one
                            // latest ancestor containing the latest update for the account, and this
                            // ancestor can only be flushed once.
                            // 2) The root cannot move while replaying, so the index cannot continually
                            // find more up to date entries than the current `slot`
                            assert!(num_acceptable_failed_iterations <= 1);
                        }
                        LoadHint::Unspecified => {
                            // Because newer root can be added to the index (= not fixed),
                            // multiple flush race conditions can be observed under very rare
                            // condition, at least theoretically
                        }
                    }
                }
                LoadedAccountAccessor::Stored(None) => {
                    match load_hint {
                        LoadHint::FixedMaxRoot => {
                            // When running replay on the validator, or banking stage on the leader,
                            // it should be very rare that the storage entry doesn't exist if the
                            // entry in the accounts index is the latest version of this account.
                            //
                            // There are only a few places where the storage entry may not exist
                            // after reading the index:
                            // 1) Shrink has removed the old storage entry and rewritten to
                            // a newer storage entry
                            // 2) The `pubkey` asked for in this function is a zero-lamport account,
                            // and the storage entry holding this account qualified for zero-lamport clean.
                            //
                            // In both these cases, it should be safe to retry and recheck the accounts
                            // index indefinitely, without incrementing num_acceptable_failed_iterations.
                            // That's because if the root is fixed, there should be a bounded number
                            // of pending cleans/shrinks (depends how far behind the AccountsBackgroundService
                            // is), termination to the desired condition is guaranteed.
                            //
                            // Also note that in both cases, if we do find the storage entry,
                            // we can guarantee that the storage entry is safe to read from because
                            // we grabbed a reference to the storage entry while it was still in the
                            // storage map. This means even if the storage entry is removed from the storage
                            // map after we grabbed the storage entry, the recycler should not reset the
                            // storage entry until we drop the reference to the storage entry.
                            //
                            // eh, no code in this arm? yes!
                        }
                        LoadHint::Unspecified => {
                            // RPC get_account() may have fetched an old root from the index that was
                            // either:
                            // 1) Cleaned up by clean_accounts(), so the accounts index has been updated
                            // and the storage entries have been removed.
                            // 2) Dropped by purge_slots() because the slot was on a minor fork, which
                            // removes the slots' storage entries but doesn't purge from the accounts index
                            // (account index cleanup is left to clean for stored slots). Note that
                            // this generally is impossible to occur in the wild because the RPC
                            // should hold the slot's bank, preventing it from being purged() to
                            // begin with.
                            num_acceptable_failed_iterations += 1;
                        }
                    }
                }
            }
            #[cfg(not(test))]
            let load_limit = ABSURD_CONSECUTIVE_FAILED_ITERATIONS;

            #[cfg(test)]
            let load_limit = self.load_limit.load(Ordering::Relaxed);

            let fallback_to_slow_path = if num_acceptable_failed_iterations >= load_limit {
                // The latest version of the account existed in the index, but could not be
                // fetched from storage. This means a race occurred between this function and clean
                // accounts/purge_slots
                let message = format!(
                    "do_load() failed to get key: {} from storage, latest attempt was for \
                     slot: {}, storage_location: {:?}, load_hint: {:?}",
                    pubkey, slot, storage_location, load_hint,
                );
                datapoint_warn!("accounts_db-do_load_warn", ("warn", message, String));
                true
            } else {
                false
            };

            // Because reading from the cache/storage failed, retry from the index read
            let (new_slot, new_storage_location, maybe_account_accessor) = self
                .read_index_for_accessor_or_load_slow(
                    ancestors,
                    pubkey,
                    max_root,
                    fallback_to_slow_path,
                )?;
            // Notice the subtle `?` at previous line, we bail out pretty early if missing.

            if new_slot == slot && new_storage_location.is_store_id_equal(&storage_location) {
                // Considering that we're failed to get accessor above and further that
                // the index still returned the same (slot, store_id) tuple, offset must be same
                // too.
                assert!(new_storage_location.is_offset_equal(&storage_location));

                // If the entry was missing from the cache, that means it must have been flushed,
                // and the accounts index is always updated before cache flush, so store_id must
                // not indicate being cached at this point.
                assert!(!new_storage_location.is_cached());

                // If this is not a cache entry, then this was a minor fork slot
                // that had its storage entries cleaned up by purge_slots() but hasn't been
                // cleaned yet. That means this must be rpc access and not replay/banking at the
                // very least. Note that purge shouldn't occur even for RPC as caller must hold all
                // of ancestor slots..
                assert_eq!(load_hint, LoadHint::Unspecified);

                // Everything being assert!()-ed, let's panic!() here as it's an error condition
                // after all....
                // That reasoning is based on the fact all of code-path reaching this fn
                // retry_to_get_account_accessor() must outlive the Arc<Bank> (and its all
                // ancestors) over this fn invocation, guaranteeing the prevention of being purged,
                // first of all.
                // For details, see the comment in AccountIndex::do_checked_scan_accounts(),
                // which is referring back here.
                panic!(
                    "Bad index entry detected ({}, {}, {:?}, {:?})",
                    pubkey, slot, storage_location, load_hint
                );
            } else if fallback_to_slow_path {
                // the above bad-index-entry check must had been checked first to retain the same
                // behavior
                return Some((
                    maybe_account_accessor.expect("must be some if clone_in_lock=true"),
                    new_slot,
                ));
            }

            slot = new_slot;
            storage_location = new_storage_location;
        }
    }

    fn do_load(
        &self,
        ancestors: &Ancestors,
        pubkey: &Pubkey,
        max_root: Option<Slot>,
        load_hint: LoadHint,
    ) -> Option<(AccountSharedData, Slot)> {
        self.do_load_with_populate_read_cache(ancestors, pubkey, max_root, load_hint, false)
    }

    /// if 'load_into_read_cache_only', then return value is meaningless.
    ///   The goal is to get the account into the read-only cache.
    fn do_load_with_populate_read_cache(
        &self,
        ancestors: &Ancestors,
        pubkey: &Pubkey,
        max_root: Option<Slot>,
        load_hint: LoadHint,
        load_into_read_cache_only: bool,
    ) -> Option<(AccountSharedData, Slot)> {
        #[cfg(not(test))]
        assert!(max_root.is_none());

        let (slot, storage_location, _maybe_account_accesor) =
            self.read_index_for_accessor_or_load_slow(ancestors, pubkey, max_root, false)?;
        // Notice the subtle `?` at previous line, we bail out pretty early if missing.

        if self.caching_enabled {
            let in_write_cache = storage_location.is_cached();
            if !load_into_read_cache_only {
                if !in_write_cache {
                    let result = self.read_only_accounts_cache.load(*pubkey, slot);
                    if let Some(account) = result {
                        return Some((account, slot));
                    }
                }
            } else {
                // goal is to load into read cache
                if in_write_cache {
                    // no reason to load in read cache. already in write cache
                    return None;
                }
                if self.read_only_accounts_cache.in_cache(pubkey, slot) {
                    // already in read cache
                    return None;
                }
            }
        }

        let (mut account_accessor, slot) = self.retry_to_get_account_accessor(
            slot,
            storage_location,
            ancestors,
            pubkey,
            max_root,
            load_hint,
        )?;
        let loaded_account = account_accessor.check_and_get_loaded_account();
        let is_cached = loaded_account.is_cached();
        let account = loaded_account.take_account();

        if self.caching_enabled && !is_cached {
            /*
            We show this store into the read-only cache for account 'A' and future loads of 'A' from the read-only cache are
            safe/reflect 'A''s latest state on this fork.
            This safety holds if during replay of slot 'S', we show we only read 'A' from the write cache,
            not the read-only cache, after it's been updated in replay of slot 'S'.
            Assume for contradiction this is not true, and we read 'A' from the read-only cache *after* it had been updated in 'S'.
            This means an entry '(S, A)' was added to the read-only cache after 'A' had been updated in 'S'.
            Now when '(S, A)' was being added to the read-only cache, it must have been true that  'is_cache == false',
            which means '(S', A)' does not exist in the write cache yet.
            However, by the assumption for contradiction above ,  'A' has already been updated in 'S' which means '(S, A)'
            must exist in the write cache, which is a contradiction.
            */
            self.read_only_accounts_cache
                .store(*pubkey, slot, account.clone());
        }
        Some((account, slot))
    }

    pub fn load_account_hash(
        &self,
        ancestors: &Ancestors,
        pubkey: &Pubkey,
        max_root: Option<Slot>,
        load_hint: LoadHint,
    ) -> Option<Hash> {
        let (slot, storage_location, _maybe_account_accesor) =
            self.read_index_for_accessor_or_load_slow(ancestors, pubkey, max_root, false)?;
        // Notice the subtle `?` at previous line, we bail out pretty early if missing.

        let (mut account_accessor, _) = self.retry_to_get_account_accessor(
            slot,
            storage_location,
            ancestors,
            pubkey,
            max_root,
            load_hint,
        )?;
        let loaded_account = account_accessor.check_and_get_loaded_account();
        Some(loaded_account.loaded_hash())
    }

    fn get_account_accessor<'a>(
        &'a self,
        slot: Slot,
        pubkey: &'a Pubkey,
        storage_location: &StorageLocation,
    ) -> LoadedAccountAccessor<'a> {
        match storage_location {
            StorageLocation::Cached => {
                let maybe_cached_account = self.accounts_cache.load(slot, pubkey).map(Cow::Owned);
                LoadedAccountAccessor::Cached(maybe_cached_account)
            }
            StorageLocation::AppendVec(store_id, offset) => {
                let maybe_storage_entry = self
                    .storage
                    .get_account_storage_entry(slot, *store_id)
                    .map(|account_storage_entry| (account_storage_entry, *offset));
                LoadedAccountAccessor::Stored(maybe_storage_entry)
            }
        }
    }

    fn try_recycle_and_insert_store(
        &self,
        slot: Slot,
        min_size: u64,
        max_size: u64,
    ) -> Option<Arc<AccountStorageEntry>> {
        let store = self.try_recycle_store(slot, min_size, max_size)?;
        self.insert_store(slot, store.clone());
        Some(store)
    }

    fn try_recycle_store(
        &self,
        slot: Slot,
        min_size: u64,
        max_size: u64,
    ) -> Option<Arc<AccountStorageEntry>> {
        let mut max = 0;
        let mut min = std::u64::MAX;
        let mut avail = 0;
        let mut recycle_stores = self.recycle_stores.write().unwrap();
        for (i, (_recycled_time, store)) in recycle_stores.iter().enumerate() {
            if Arc::strong_count(store) == 1 {
                max = std::cmp::max(store.accounts.capacity(), max);
                min = std::cmp::min(store.accounts.capacity(), min);
                avail += 1;

                if store.accounts.capacity() >= min_size && store.accounts.capacity() < max_size {
                    let ret = recycle_stores.remove_entry(i);
                    drop(recycle_stores);
                    let old_id = ret.append_vec_id();
                    ret.recycle(slot, self.next_id());
                    debug!(
                        "recycling store: {} {:?} old_id: {}",
                        ret.append_vec_id(),
                        ret.get_path(),
                        old_id
                    );
                    return Some(ret);
                }
            }
        }
        debug!(
            "no recycle stores max: {} min: {} len: {} looking: {}, {} avail: {}",
            max,
            min,
            recycle_stores.entry_count(),
            min_size,
            max_size,
            avail,
        );
        None
    }

    fn find_storage_candidate(&self, slot: Slot, size: usize) -> Arc<AccountStorageEntry> {
        let mut create_extra = false;
        let mut get_slot_stores = Measure::start("get_slot_stores");
        let slot_stores_lock = self.storage.get_slot_stores(slot);
        get_slot_stores.stop();
        self.stats
            .store_get_slot_store
            .fetch_add(get_slot_stores.as_us(), Ordering::Relaxed);
        let mut find_existing = Measure::start("find_existing");
        if let Some(slot_stores_lock) = slot_stores_lock {
            let slot_stores = slot_stores_lock.read().unwrap();
            if !slot_stores.is_empty() {
                if slot_stores.len() <= self.min_num_stores {
                    let mut total_accounts = 0;
                    for store in slot_stores.values() {
                        total_accounts += store.count();
                    }

                    // Create more stores so that when scanning the storage all CPUs have work
                    if (total_accounts / 16) >= slot_stores.len() {
                        create_extra = true;
                    }
                }

                // pick an available store at random by iterating from a random point
                let to_skip = thread_rng().gen_range(0, slot_stores.len());

                for (i, store) in slot_stores.values().cycle().skip(to_skip).enumerate() {
                    if store.try_available() {
                        let ret = store.clone();
                        drop(slot_stores);
                        if create_extra {
                            if self
                                .try_recycle_and_insert_store(slot, size as u64, std::u64::MAX)
                                .is_none()
                            {
                                self.stats
                                    .create_store_count
                                    .fetch_add(1, Ordering::Relaxed);
                                self.create_and_insert_store(slot, self.file_size, "store extra");
                            } else {
                                self.stats
                                    .recycle_store_count
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        find_existing.stop();
                        self.stats
                            .store_find_existing
                            .fetch_add(find_existing.as_us(), Ordering::Relaxed);
                        return ret;
                    }
                    // looked at every store, bail...
                    if i == slot_stores.len() {
                        break;
                    }
                }
            }
        }
        find_existing.stop();
        self.stats
            .store_find_existing
            .fetch_add(find_existing.as_us(), Ordering::Relaxed);

        let store = if let Some(store) = self.try_recycle_store(slot, size as u64, std::u64::MAX) {
            self.stats
                .recycle_store_count
                .fetch_add(1, Ordering::Relaxed);
            store
        } else {
            self.stats
                .create_store_count
                .fetch_add(1, Ordering::Relaxed);
            self.create_store(slot, self.file_size, "store", &self.paths)
        };

        // try_available is like taking a lock on the store,
        // preventing other threads from using it.
        // It must succeed here and happen before insert,
        // otherwise another thread could also grab it from the index.
        assert!(store.try_available());
        self.insert_store(slot, store.clone());
        store
    }

    pub(crate) fn page_align(size: u64) -> u64 {
        (size + (PAGE_SIZE - 1)) & !(PAGE_SIZE - 1)
    }

    fn has_space_available(&self, slot: Slot, size: u64) -> bool {
        let slot_storage = self.storage.get_slot_stores(slot).unwrap();
        let slot_storage_r = slot_storage.read().unwrap();
        for (_id, store) in slot_storage_r.iter() {
            if store.status() == AccountStorageStatus::Available
                && (store.accounts.capacity() - store.accounts.len() as u64) > size
            {
                return true;
            }
        }
        false
    }

    fn create_store(
        &self,
        slot: Slot,
        size: u64,
        from: &str,
        paths: &[PathBuf],
    ) -> Arc<AccountStorageEntry> {
        let path_index = thread_rng().gen_range(0, paths.len());
        let store = Arc::new(self.new_storage_entry(
            slot,
            Path::new(&paths[path_index]),
            Self::page_align(size),
        ));

        debug!(
            "creating store: {} slot: {} len: {} size: {} from: {} path: {:?}",
            store.append_vec_id(),
            slot,
            store.accounts.len(),
            store.accounts.capacity(),
            from,
            store.accounts.get_path()
        );

        store
    }

    fn create_and_insert_store(
        &self,
        slot: Slot,
        size: u64,
        from: &str,
    ) -> Arc<AccountStorageEntry> {
        self.create_and_insert_store_with_paths(slot, size, from, &self.paths)
    }

    fn create_and_insert_store_with_paths(
        &self,
        slot: Slot,
        size: u64,
        from: &str,
        paths: &[PathBuf],
    ) -> Arc<AccountStorageEntry> {
        let store = self.create_store(slot, size, from, paths);
        let store_for_index = store.clone();

        self.insert_store(slot, store_for_index);
        store
    }

    fn insert_store(&self, slot: Slot, store: Arc<AccountStorageEntry>) {
        let slot_storages: SlotStores = self.storage.get_slot_stores(slot).unwrap_or_else(||
            // DashMap entry.or_insert() returns a RefMut, essentially a write lock,
            // which is dropped after this block ends, minimizing time held by the lock.
            // However, we still want to persist the reference to the `SlotStores` behind
            // the lock, hence we clone it out, (`SlotStores` is an Arc so is cheap to clone).
            self.storage
                .map
                .entry(slot)
                .or_insert(Arc::new(RwLock::new(HashMap::new())))
                .clone());

        assert!(slot_storages
            .write()
            .unwrap()
            .insert(store.append_vec_id(), store)
            .is_none());
    }

    pub fn create_drop_bank_callback(
        &self,
        pruned_banks_sender: DroppedSlotsSender,
    ) -> SendDroppedBankCallback {
        self.is_bank_drop_callback_enabled
            .store(true, Ordering::Release);
        SendDroppedBankCallback::new(pruned_banks_sender)
    }

    /// This should only be called after the `Bank::drop()` runs in bank.rs, See BANK_DROP_SAFETY
    /// comment below for more explanation.
    ///   * `is_serialized_with_abs` - indicates whehter this call runs sequentially with all other
    ///        accounts_db relevant calls, such as shrinking, purging etc., in account background
    ///        service.
    pub fn purge_slot(&self, slot: Slot, bank_id: BankId, is_serialized_with_abs: bool) {
        if self.is_bank_drop_callback_enabled.load(Ordering::Acquire) && !is_serialized_with_abs {
            panic!(
                "bad drop callpath detected; Bank::drop() must run serially with other logic in
                ABS like clean_accounts()"
            )
        }

        // BANK_DROP_SAFETY: Because this function only runs once the bank is dropped,
        // we know that there are no longer any ongoing scans on this bank, because scans require
        // and hold a reference to the bank at the tip of the fork they're scanning. Hence it's
        // safe to remove this bank_id from the `removed_bank_ids` list at this point.
        if self
            .accounts_index
            .removed_bank_ids
            .lock()
            .unwrap()
            .remove(&bank_id)
        {
            // If this slot was already cleaned up, no need to do any further cleans
            return;
        }

        self.purge_slots(std::iter::once(&slot));
    }

    fn recycle_slot_stores(
        &self,
        total_removed_storage_entries: usize,
        slot_stores: &[SlotStores],
    ) -> u64 {
        let mut recycled_count = 0;

        let mut recycle_stores_write_elapsed = Measure::start("recycle_stores_write_elapsed");
        let mut recycle_stores = self.recycle_stores.write().unwrap();
        recycle_stores_write_elapsed.stop();

        for slot_entries in slot_stores {
            let entry = slot_entries.read().unwrap();
            for (_store_id, stores) in entry.iter() {
                if recycle_stores.entry_count() > MAX_RECYCLE_STORES {
                    let dropped_count = total_removed_storage_entries - recycled_count;
                    self.stats
                        .dropped_stores
                        .fetch_add(dropped_count as u64, Ordering::Relaxed);
                    return recycle_stores_write_elapsed.as_us();
                }
                recycle_stores.add_entry(stores.clone());
                recycled_count += 1;
            }
        }
        recycle_stores_write_elapsed.as_us()
    }

    /// Purges every slot in `removed_slots` from both the cache and storage. This includes
    /// entries in the accounts index, cache entries, and any backing storage entries.
    pub(crate) fn purge_slots_from_cache_and_store<'a>(
        &self,
        removed_slots: impl Iterator<Item = &'a Slot> + Clone,
        purge_stats: &PurgeStats,
        log_accounts: bool,
    ) {
        let mut remove_cache_elapsed_across_slots = 0;
        let mut num_cached_slots_removed = 0;
        let mut total_removed_cached_bytes = 0;
        if log_accounts {
            if let Some(min) = removed_slots.clone().min() {
                info!(
                    "purge_slots_from_cache_and_store: {:?}",
                    self.get_pubkey_hash_for_slot(*min).0
                );
            }
        }
        for remove_slot in removed_slots {
            // This function is only currently safe with respect to `flush_slot_cache()` because
            // both functions run serially in AccountsBackgroundService.
            let mut remove_cache_elapsed = Measure::start("remove_cache_elapsed");
            // Note: we cannot remove this slot from the slot cache until we've removed its
            // entries from the accounts index first. This is because `scan_accounts()` relies on
            // holding the index lock, finding the index entry, and then looking up the entry
            // in the cache. If it fails to find that entry, it will panic in `get_loaded_account()`
            if let Some(slot_cache) = self.accounts_cache.slot_cache(*remove_slot) {
                // If the slot is still in the cache, remove the backing storages for
                // the slot and from the Accounts Index
                num_cached_slots_removed += 1;
                total_removed_cached_bytes += slot_cache.total_bytes();
                self.purge_slot_cache(*remove_slot, slot_cache);
                remove_cache_elapsed.stop();
                remove_cache_elapsed_across_slots += remove_cache_elapsed.as_us();
                // Nobody else shoud have removed the slot cache entry yet
                assert!(self.accounts_cache.remove_slot(*remove_slot).is_some());
            } else {
                self.purge_slot_storage(*remove_slot, purge_stats);
            }
            // It should not be possible that a slot is neither in the cache or storage. Even in
            // a slot with all ticks, `Bank::new_from_parent()` immediately stores some sysvars
            // on bank creation.
        }

        purge_stats
            .remove_cache_elapsed
            .fetch_add(remove_cache_elapsed_across_slots, Ordering::Relaxed);
        purge_stats
            .num_cached_slots_removed
            .fetch_add(num_cached_slots_removed, Ordering::Relaxed);
        purge_stats
            .total_removed_cached_bytes
            .fetch_add(total_removed_cached_bytes, Ordering::Relaxed);
    }

    /// Purge the backing storage entries for the given slot, does not purge from
    /// the cache!
    fn purge_dead_slots_from_storage<'a>(
        &'a self,
        removed_slots: impl Iterator<Item = &'a Slot> + Clone,
        purge_stats: &PurgeStats,
    ) {
        // Check all slots `removed_slots` are no longer "relevant" roots.
        // Note that the slots here could have been rooted slots, but if they're passed here
        // for removal it means:
        // 1) All updates in that old root have been outdated by updates in newer roots
        // 2) Those slots/roots should have already been purged from the accounts index root
        // tracking metadata via `accounts_index.clean_dead_slot()`.
        let mut safety_checks_elapsed = Measure::start("safety_checks_elapsed");
        assert!(self
            .accounts_index
            .get_rooted_from_list(removed_slots.clone())
            .is_empty());
        safety_checks_elapsed.stop();
        purge_stats
            .safety_checks_elapsed
            .fetch_add(safety_checks_elapsed.as_us(), Ordering::Relaxed);

        let mut total_removed_storage_entries = 0;
        let mut total_removed_stored_bytes = 0;
        let mut all_removed_slot_storages = vec![];

        let mut remove_storage_entries_elapsed = Measure::start("remove_storage_entries_elapsed");
        for remove_slot in removed_slots {
            // Remove the storage entries and collect some metrics
            if let Some((_, slot_storages_to_be_removed)) = self.storage.map.remove(remove_slot) {
                {
                    let r_slot_removed_storages = slot_storages_to_be_removed.read().unwrap();
                    total_removed_storage_entries += r_slot_removed_storages.len();
                    total_removed_stored_bytes += r_slot_removed_storages
                        .values()
                        .map(|i| i.accounts.capacity())
                        .sum::<u64>();
                }
                all_removed_slot_storages.push(slot_storages_to_be_removed.clone());
            }
        }
        remove_storage_entries_elapsed.stop();
        let num_stored_slots_removed = all_removed_slot_storages.len();

        let recycle_stores_write_elapsed =
            self.recycle_slot_stores(total_removed_storage_entries, &all_removed_slot_storages);

        let mut drop_storage_entries_elapsed = Measure::start("drop_storage_entries_elapsed");
        // Backing mmaps for removed storages entries explicitly dropped here outside
        // of any locks
        drop(all_removed_slot_storages);
        drop_storage_entries_elapsed.stop();
        purge_stats
            .remove_storage_entries_elapsed
            .fetch_add(remove_storage_entries_elapsed.as_us(), Ordering::Relaxed);
        purge_stats
            .drop_storage_entries_elapsed
            .fetch_add(drop_storage_entries_elapsed.as_us(), Ordering::Relaxed);
        purge_stats
            .num_stored_slots_removed
            .fetch_add(num_stored_slots_removed, Ordering::Relaxed);
        purge_stats
            .total_removed_storage_entries
            .fetch_add(total_removed_storage_entries, Ordering::Relaxed);
        purge_stats
            .total_removed_stored_bytes
            .fetch_add(total_removed_stored_bytes, Ordering::Relaxed);
        purge_stats
            .recycle_stores_write_elapsed
            .fetch_add(recycle_stores_write_elapsed, Ordering::Relaxed);
    }

    fn purge_slot_cache(&self, purged_slot: Slot, slot_cache: SlotCache) {
        let mut purged_slot_pubkeys: HashSet<(Slot, Pubkey)> = HashSet::new();
        let pubkey_to_slot_set: Vec<(Pubkey, Slot)> = slot_cache
            .iter()
            .map(|account| {
                purged_slot_pubkeys.insert((purged_slot, *account.key()));
                (*account.key(), purged_slot)
            })
            .collect();
        self.purge_slot_cache_pubkeys(purged_slot, purged_slot_pubkeys, pubkey_to_slot_set, true);
    }

    fn purge_slot_cache_pubkeys(
        &self,
        purged_slot: Slot,
        purged_slot_pubkeys: HashSet<(Slot, Pubkey)>,
        pubkey_to_slot_set: Vec<(Pubkey, Slot)>,
        is_dead: bool,
    ) {
        // Slot purged from cache should not exist in the backing store
        assert!(self.storage.get_slot_stores(purged_slot).is_none());
        let num_purged_keys = pubkey_to_slot_set.len();
        let reclaims = self.purge_keys_exact(pubkey_to_slot_set.iter());
        assert_eq!(reclaims.len(), num_purged_keys);
        if is_dead {
            self.remove_dead_slots_metadata(
                std::iter::once(&purged_slot),
                purged_slot_pubkeys,
                None,
            );
        }
    }

    fn purge_slot_storage(&self, remove_slot: Slot, purge_stats: &PurgeStats) {
        // Because AccountsBackgroundService synchronously flushes from the accounts cache
        // and handles all Bank::drop() (the cleanup function that leads to this
        // function call), then we don't need to worry above an overlapping cache flush
        // with this function call. This means, if we get into this case, we can be
        // confident that the entire state for this slot has been flushed to the storage
        // already.
        let mut scan_storages_elasped = Measure::start("scan_storages_elasped");
        type ScanResult = ScanStorageResult<Pubkey, Arc<Mutex<HashSet<(Pubkey, Slot)>>>>;
        let scan_result: ScanResult = self.scan_account_storage(
            remove_slot,
            |loaded_account: LoadedAccount| Some(*loaded_account.pubkey()),
            |accum: &Arc<Mutex<HashSet<(Pubkey, Slot)>>>, loaded_account: LoadedAccount| {
                accum
                    .lock()
                    .unwrap()
                    .insert((*loaded_account.pubkey(), remove_slot));
            },
        );
        scan_storages_elasped.stop();
        purge_stats
            .scan_storages_elapsed
            .fetch_add(scan_storages_elasped.as_us(), Ordering::Relaxed);

        let mut purge_accounts_index_elapsed = Measure::start("purge_accounts_index_elapsed");
        let reclaims = match scan_result {
            ScanStorageResult::Cached(_) => {
                panic!("Should not see cached keys in this `else` branch, since we checked this slot did not exist in the cache above");
            }
            ScanStorageResult::Stored(stored_keys) => {
                // Purge this slot from the accounts index
                self.purge_keys_exact(stored_keys.lock().unwrap().iter())
            }
        };
        purge_accounts_index_elapsed.stop();
        purge_stats
            .purge_accounts_index_elapsed
            .fetch_add(purge_accounts_index_elapsed.as_us(), Ordering::Relaxed);

        // `handle_reclaims()` should remove all the account index entries and
        // storage entries
        let mut handle_reclaims_elapsed = Measure::start("handle_reclaims_elapsed");
        // Slot should be dead after removing all its account entries
        let expected_dead_slot = Some(remove_slot);
        self.handle_reclaims(
            &reclaims,
            expected_dead_slot,
            Some((purge_stats, &mut ReclaimResult::default())),
            false,
        );
        handle_reclaims_elapsed.stop();
        purge_stats
            .handle_reclaims_elapsed
            .fetch_add(handle_reclaims_elapsed.as_us(), Ordering::Relaxed);
        // After handling the reclaimed entries, this slot's
        // storage entries should be purged from self.storage
        assert!(
            self.storage.get_slot_stores(remove_slot).is_none(),
            "slot {} is not none",
            remove_slot
        );
    }

    #[allow(clippy::needless_collect)]
    fn purge_slots<'a>(&self, slots: impl Iterator<Item = &'a Slot> + Clone) {
        // `add_root()` should be called first
        let mut safety_checks_elapsed = Measure::start("safety_checks_elapsed");
        let non_roots = slots
            // Only safe to check when there are duplicate versions of a slot
            // because ReplayStage will not make new roots before dumping the
            // duplicate slots first. Thus we will not be in a case where we
            // root slot `S`, then try to dump some other version of slot `S`, the
            // dumping has to finish first
            //
            // Also note roots are never removed via `remove_unrooted_slot()`, so
            // it's safe to filter them out here as they won't need deletion from
            // self.accounts_index.removed_bank_ids in `purge_slots_from_cache_and_store()`.
            .filter(|slot| !self.accounts_index.is_alive_root(**slot));
        safety_checks_elapsed.stop();
        self.external_purge_slots_stats
            .safety_checks_elapsed
            .fetch_add(safety_checks_elapsed.as_us(), Ordering::Relaxed);
        self.purge_slots_from_cache_and_store(non_roots, &self.external_purge_slots_stats, false);
        self.external_purge_slots_stats
            .report("external_purge_slots_stats", Some(1000));
    }