    fn test_release_locations_keep_state_nopanic() {
        let mut mechine = Machine::new();
        let mut assembler = Assembler::new().unwrap();
        let locs = mechine.acquire_locations(&mut assembler, &[WpType::I32; 10], false);

        mechine.release_locations_keep_state(&mut assembler, &locs);
    }
