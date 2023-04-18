    fn check_id(pubkey: &Pubkey) -> bool;
}

// utilities for moving into and out of Accounts
pub trait Sysvar:
    SysvarId + Default + Sized + serde::Serialize + serde::de::DeserializeOwned
{
    fn size_of() -> usize {
        bincode::serialized_size(&Self::default()).unwrap() as usize
    }
    fn from_account(account: &Account) -> Option<Self> {
        bincode::deserialize(&account.data).ok()
    }
    fn to_account(&self, account: &mut Account) -> Option<()> {
        bincode::serialize_into(&mut account.data[..], self).ok()
    }
    fn from_account_info(account_info: &AccountInfo) -> Option<Self> {
        bincode::deserialize(&account_info.data.borrow()).ok()
    }
    fn to_account_info(&self, account_info: &mut AccountInfo) -> Option<()> {
        bincode::serialize_into(&mut account_info.data.borrow_mut()[..], self).ok()
    }
    fn from_keyed_account(keyed_account: &KeyedAccount) -> Result<Self, InstructionError> {
        if !Self::check_id(keyed_account.unsigned_key()) {
            return Err(InstructionError::InvalidArgument);
        }
        Self::from_account(&*keyed_account.try_account_ref()?)
            .ok_or(InstructionError::InvalidArgument)
    }
    fn create_account(&self, lamports: u64) -> Account {
        let data_len = Self::size_of().max(bincode::serialized_size(self).unwrap() as usize);
        let mut account = Account::new(lamports, data_len, &id());
        self.to_account(&mut account).unwrap();
        account
    }
}
