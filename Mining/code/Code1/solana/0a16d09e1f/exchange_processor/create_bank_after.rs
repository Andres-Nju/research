    fn create_bank(lamports: u64) -> (Bank, Keypair) {
        let (genesis_config, mint_keypair) = create_genesis_config(lamports);
        let mut bank = Bank::new(&genesis_config);
        bank.add_static_program("exchange_program", id(), process_instruction);
        (bank, mint_keypair)
    }
