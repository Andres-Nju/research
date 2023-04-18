    fn test_duplicate_transaction_signature() {
        let mint = Mint::new(1);
        let bank = Bank::new(&mint);
        let sig = Signature::default();
        assert!(
            bank.reserve_signature_with_last_id(&sig, &mint.last_id())
                .is_ok()
        );
        assert_eq!(
            bank.reserve_signature_with_last_id(&sig, &mint.last_id()),
            Err(BankError::DuplicateSiganture(sig))
        );
    }
