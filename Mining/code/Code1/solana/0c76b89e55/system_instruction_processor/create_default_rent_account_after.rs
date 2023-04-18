    fn create_default_rent_account() -> RefCell<Account> {
        RefCell::new(sysvar::rent::create_account(1, &Rent::free()))
    }
