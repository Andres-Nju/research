File_Code/parity-ethereum/f442665c46/externalities/externalities_after.rs --- 1/2 --- Rust
595                         let mut ext = Externalities::new(state, &setup.env_info, &setup.machine, 0, get_test_origin(), &mut setup.sub_state, OutputPolic 595                         let mut ext = Externalities::new(state, &setup.env_info, &setup.machine, &setup.schedule, 0, get_test_origin(), &mut setup.sub_s
    y::InitContract(None), &mut tracer, &mut vm_tracer, false);                                                                                                  tate, OutputPolicy::InitContract(None), &mut tracer, &mut vm_tracer, false);

File_Code/parity-ethereum/f442665c46/externalities/externalities_after.rs --- 2/2 --- Rust
615                         let mut ext = Externalities::new(state, &setup.env_info, &setup.machine, 0, get_test_origin(), &mut setup.sub_state, OutputPolic 615                         let mut ext = Externalities::new(state, &setup.env_info, &setup.machine, &setup.schedule, 0, get_test_origin(), &mut setup.sub_s
    y::InitContract(None), &mut tracer, &mut vm_tracer, false);                                                                                                  tate, OutputPolicy::InitContract(None), &mut tracer, &mut vm_tracer, false);

