    fn test_fees_create_account() {
        let lamports = 42;
        let account = create_account(lamports, &RentCalculator::default());
        let rent = Rent::from_account(&account).unwrap();
        assert_eq!(rent.rent_calculator, RentCalculator::default());
    }
