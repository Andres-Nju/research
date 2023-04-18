    fn test_release_locations_keep_state_nopanic() {
        let mut machine = Machine::new();
        let mut assembler = Assembler::new().unwrap();
        let locs = machine.acquire_locations(&mut assembler, &[WpType::I32; 10], false);

        machine.release_locations_keep_state(&mut assembler, &locs);
    }
