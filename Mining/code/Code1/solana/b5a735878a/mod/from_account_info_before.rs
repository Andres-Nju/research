    fn from_account_info(account_info: &AccountInfo) -> Option<Self> {
        bincode::deserialize(&account_info.data.borrow()).ok()
    }
