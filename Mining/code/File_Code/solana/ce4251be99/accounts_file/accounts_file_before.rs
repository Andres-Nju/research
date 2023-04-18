use {
    crate::{append_vec::*, storable_accounts::StorableAccounts},
    solana_sdk::{account::ReadableAccount, clock::Slot, hash::Hash, pubkey::Pubkey},
    std::{borrow::Borrow, io, path::PathBuf},
};

#[derive(Debug)]
/// An enum for accessing an accounts file which can be implemented
/// under different formats.
pub enum AccountsFile {
    AppendVec(AppendVec),
}

impl AccountsFile {
    /// By default, all AccountsFile will remove its underlying file on
    /// drop.  Calling this function to disable such behavior for this
    /// instance.
    pub fn set_no_remove_on_drop(&mut self) {
        match self {
            Self::AppendVec(av) => av.set_no_remove_on_drop(),
        }
    }

    pub fn flush(&self) -> io::Result<()> {
        match self {
            Self::AppendVec(av) => av.flush(),
        }
    }

    pub fn reset(&self) {
        match self {
            Self::AppendVec(av) => av.reset(),
        }
    }

    pub fn remaining_bytes(&self) -> u64 {
        match self {
            Self::AppendVec(av) => av.remaining_bytes(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::AppendVec(av) => av.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::AppendVec(av) => av.is_empty(),
        }
    }

    pub fn capacity(&self) -> u64 {
        match self {
            Self::AppendVec(av) => av.capacity(),
        }
    }

    pub fn file_name(slot: Slot, id: impl std::fmt::Display) -> String {
        format!("{slot}.{id}")
    }

    /// Return (account metadata, next_index) pair for the account at the
    /// specified `index` if any.  Otherwise return None.   Also return the
    /// index of the next entry.
    pub fn get_account(&self, index: usize) -> Option<(StoredAccountMeta<'_>, usize)> {
        match self {
            Self::AppendVec(av) => av.get_account(index),
        }
    }

    pub fn account_matches_owners(
        &self,
        offset: usize,
        owners: &[&Pubkey],
    ) -> Result<(), MatchAccountOwnerError> {
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
}

pub struct AccountsFileIter<'a> {
    file_entry: &'a AccountsFile,
    offset: usize,
}

impl<'a> AccountsFileIter<'a> {
    pub fn new(file_entry: &'a AccountsFile) -> Self {
        Self {
            file_entry,
            offset: 0,
        }
    }
}

impl<'a> Iterator for AccountsFileIter<'a> {
    type Item = StoredAccountMeta<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((account, next_offset)) = self.file_entry.get_account(self.offset) {
            self.offset = next_offset;
            Some(account)
        } else {
            None
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::accounts_file::AccountsFile;
    impl AccountsFile {
        pub(crate) fn set_current_len_for_tests(&self, len: usize) {
            match self {
                Self::AppendVec(av) => av.set_current_len_for_tests(len),
            }
        }
    }
}
