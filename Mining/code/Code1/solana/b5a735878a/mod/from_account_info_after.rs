    fn from_account_info(account_info: &AccountInfo) -> Result<Self, ProgramError> {
        bincode::deserialize(&account_info.data.borrow()).map_err(|_| ProgramError::InvalidArgument)
    }
