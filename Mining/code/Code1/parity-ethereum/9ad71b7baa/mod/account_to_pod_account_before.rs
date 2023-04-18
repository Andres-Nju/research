	fn account_to_pod_account(&self, account: &Account, address: &Address) -> Result<PodAccount, Error> {
		let mut pod_storage = BTreeMap::new();
		let addr_hash = account.address_hash(address);
		let accountdb = self.factories.accountdb.readonly(self.db.as_hash_db(), addr_hash);
		let root = account.base_storage_root();

		let accountdb = &accountdb.as_hash_db();
		let trie = self.factories.trie.readonly(accountdb, &root)?;
		for o_kv in trie.iter()? {
			if let Ok((key, val)) = o_kv {
				pod_storage.insert(key[..].into(), U256::from(&val[..]).into());
			}
		}

		let mut pod_account = PodAccount::from_account(&account);
		// cached one first
		pod_storage.append(&mut pod_account.storage);
		pod_account.storage = pod_storage;
		Ok(pod_account)
	}
