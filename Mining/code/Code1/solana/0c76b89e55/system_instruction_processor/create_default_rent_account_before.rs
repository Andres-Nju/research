    fn create_default_rent_account() -> RefCell<Account> {
        RefCell::new(sysvar::recent_blockhashes::create_account_with_data(
            1,
            vec![(0u64, &Hash::default()); 32].into_iter(),
        ))
    }
