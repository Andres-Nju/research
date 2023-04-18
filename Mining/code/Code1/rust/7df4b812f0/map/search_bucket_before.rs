    pub fn search_bucket<F>(self, hash: u64, is_match: F) -> RawEntryMut<'a, K, V, S>
        where for<'b> F: FnMut(&'b K) -> bool,
    {
        self.search(hash, is_match, false)
    }
}

impl<'a, K, V, S> RawEntryBuilder<'a, K, V, S>
    where S: BuildHasher,
{
    /// Access an entry by key.
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_key<Q: ?Sized>(self, k: &Q) -> Option<(&'a K, &'a V)>
        where K: Borrow<Q>,
              Q: Hash + Eq
    {
        let mut hasher = self.map.hash_builder.build_hasher();
        k.hash(&mut hasher);
        self.from_key_hashed_nocheck(hasher.finish(), k)
    }

    /// Access an entry by a key and its hash.
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_key_hashed_nocheck<Q: ?Sized>(self, hash: u64, k: &Q) -> Option<(&'a K, &'a V)>
        where K: Borrow<Q>,
              Q: Hash + Eq

    {
        self.from_hash(hash, |q| q.borrow().eq(k))
    }

    fn search<F>(self, hash: u64, is_match: F, compare_hashes: bool) -> Option<(&'a K, &'a V)>
        where F: FnMut(&K) -> bool
    {
        match search_hashed_nonempty(&self.map.table,
                                     SafeHash::new(hash),
                                     is_match,
                                     compare_hashes) {
            InternalEntry::Occupied { elem } => Some(elem.into_refs()),
            InternalEntry::Vacant { .. } => None,
            InternalEntry::TableIsEmpty => unreachable!(),
        }
    }

    /// Access an entry by hash.
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_hash<F>(self, hash: u64, is_match: F) -> Option<(&'a K, &'a V)>
        where F: FnMut(&K) -> bool
    {
        self.search(hash, is_match, true)
    }

    /// Search possible locations for an element with hash `hash` until `is_match` returns true for
    /// one of them. There is no guarantee that all keys passed to `is_match` will have the provided
    /// hash.
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn search_bucket<F>(self, hash: u64, is_match: F) -> Option<(&'a K, &'a V)>
        where F: FnMut(&K) -> bool
    {
        self.search(hash, is_match, false)
    }
}

impl<'a, K, V, S> RawEntryMut<'a, K, V, S> {
    /// Ensures a value is in the entry by inserting the default if empty, and returns
    /// mutable references to the key and value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(hash_raw_entry)]
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// map.raw_entry_mut().from_key("poneyland").or_insert("poneyland", 3);
    /// assert_eq!(map["poneyland"], 3);
    ///
    /// *map.raw_entry_mut().from_key("poneyland").or_insert("poneyland", 10).1 *= 2;
    /// assert_eq!(map["poneyland"], 6);
    /// ```
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn or_insert(self, default_key: K, default_val: V) -> (&'a mut K, &'a mut V)
        where K: Hash,
              S: BuildHasher,
    {
        match self {
            RawEntryMut::Occupied(entry) => entry.into_key_value(),
            RawEntryMut::Vacant(entry) => entry.insert(default_key, default_val),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty,
    /// and returns mutable references to the key and value in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(hash_raw_entry)]
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, String> = HashMap::new();
    ///
    /// map.raw_entry_mut().from_key("poneyland").or_insert_with(|| {
    ///     ("poneyland", "hoho".to_string())
    /// });
    ///
    /// assert_eq!(map["poneyland"], "hoho".to_string());
    /// ```
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn or_insert_with<F>(self, default: F) -> (&'a mut K, &'a mut V)
        where F: FnOnce() -> (K, V),
              K: Hash,
              S: BuildHasher,
    {
        match self {
            RawEntryMut::Occupied(entry) => entry.into_key_value(),
            RawEntryMut::Vacant(entry) => {
                let (k, v) = default();
                entry.insert(k, v)
            }
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(hash_raw_entry)]
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// map.raw_entry_mut()
    ///    .from_key("poneyland")
    ///    .and_modify(|_k, v| { *v += 1 })
    ///    .or_insert("poneyland", 42);
    /// assert_eq!(map["poneyland"], 42);
    ///
    /// map.raw_entry_mut()
    ///    .from_key("poneyland")
    ///    .and_modify(|_k, v| { *v += 1 })
    ///    .or_insert("poneyland", 0);
    /// assert_eq!(map["poneyland"], 43);
    /// ```
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn and_modify<F>(self, f: F) -> Self
        where F: FnOnce(&mut K, &mut V)
    {
        match self {
            RawEntryMut::Occupied(mut entry) => {
                {
                    let (k, v) = entry.get_key_value_mut();
                    f(k, v);
                }
                RawEntryMut::Occupied(entry)
            },
            RawEntryMut::Vacant(entry) => RawEntryMut::Vacant(entry),
        }
    }
}

impl<'a, K, V> RawOccupiedEntryMut<'a, K, V> {
    /// Gets a reference to the key in the entry.
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn key(&self) -> &K {
        self.elem.read().0
    }

    /// Gets a mutable reference to the key in the entry.
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn key_mut(&mut self) -> &mut K {
        self.elem.read_mut().0
    }

    /// Converts the entry into a mutable reference to the key in the entry
    /// with a lifetime bound to the map itself.
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn into_key(self) -> &'a mut K {
        self.elem.into_mut_refs().0
    }
