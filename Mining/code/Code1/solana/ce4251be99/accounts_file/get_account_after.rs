    pub fn get_account(&self, index: usize) -> Option<(StoredAccountMeta<'_>, usize)> {
        match self {
            Self::AppendVec(av) => av.get_account(index),
        }
    }

    pub fn account_matches_owners(
        &self,
        offset: usize,
        owners: &[&Pubkey],
    ) -> Result<usize, MatchAccountOwnerError> {
        match self {
            Self::AppendVec(av) => av.account_matches_owners(offset, owners),
        }
    }

    /// Return the path of the underlying account file.
    pub fn get_path(&self) -> PathBuf {
        match self {
            Self::AppendVec(av) => av.get_path(),
        }
    }

    /// Return iterator for account metadata
    pub fn account_iter(&self) -> AccountsFileIter {
        AccountsFileIter::new(self)
    }

    /// Return a vector of account metadata for each account, starting from `offset`.
    pub fn accounts(&self, offset: usize) -> Vec<StoredAccountMeta> {
        match self {
            Self::AppendVec(av) => av.accounts(offset),
        }
    }

    /// Copy each account metadata, account and hash to the internal buffer.
    /// If there is no room to write the first entry, None is returned.
    /// Otherwise, returns the starting offset of each account metadata.
    /// Plus, the final return value is the offset where the next entry would be appended.
    /// So, return.len() is 1 + (number of accounts written)
    /// After each account is appended, the internal `current_len` is updated
    /// and will be available to other threads.
    pub fn append_accounts<
        'a,
        'b,
        T: ReadableAccount + Sync,
        U: StorableAccounts<'a, T>,
        V: Borrow<Hash>,
    >(
        &self,
        accounts: &StorableAccountsWithHashesAndWriteVersions<'a, 'b, T, U, V>,
        skip: usize,
    ) -> Option<Vec<usize>> {
        match self {
            Self::AppendVec(av) => av.append_accounts(accounts, skip),
        }
    }
