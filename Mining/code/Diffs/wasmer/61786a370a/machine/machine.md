File_Code/wasmer/61786a370a/machine/machine_after.rs --- Rust
430         let mut mechine = Machine::new();                                                                                                                430         let mut machine = Machine::new();
431         let mut assembler = Assembler::new().unwrap();                                                                                                   431         let mut assembler = Assembler::new().unwrap();
432         let locs = mechine.acquire_locations(&mut assembler, &[WpType::I32; 10], false);                                                                 432         let locs = machine.acquire_locations(&mut assembler, &[WpType::I32; 10], false);
433                                                                                                                                                          433 
434         mechine.release_locations_keep_state(&mut assembler, &locs);                                                                                     434         machine.release_locations_keep_state(&mut assembler, &locs);

