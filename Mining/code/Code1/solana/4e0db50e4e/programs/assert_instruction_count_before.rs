fn assert_instruction_count() {
    solana_logger::setup();

    let mut programs = Vec::new();
    #[cfg(feature = "bpf_c")]
    {
        programs.extend_from_slice(&[
            ("alloc", 1137),
            ("bpf_to_bpf", 13),
            ("multiple_static", 8),
            ("noop", 42),
            ("noop++", 42),
            ("relative_call", 10),
            ("sanity", 174),
            ("sanity++", 174),
            ("sha", 694),
            ("struct_pass", 8),
            ("struct_ret", 22),
        ]);
    }
    #[cfg(feature = "bpf_rust")]
    {
        programs.extend_from_slice(&[
            ("solana_bpf_rust_128bit", 572),
            ("solana_bpf_rust_alloc", 8906),
            ("solana_bpf_rust_custom_heap", 539),
            ("solana_bpf_rust_dep_crate", 2),
            ("solana_bpf_rust_external_spend", 521),
            ("solana_bpf_rust_iter", 724),
            ("solana_bpf_rust_many_args", 237),
            ("solana_bpf_rust_mem", 3143),
            ("solana_bpf_rust_membuiltins", 4069),
            ("solana_bpf_rust_noop", 495),
            ("solana_bpf_rust_param_passing", 46),
            ("solana_bpf_rust_rand", 498),
            ("solana_bpf_rust_sanity", 917),
            ("solana_bpf_rust_sha", 29099),
        ]);
    }

    let mut passed = true;
    println!("\n  {:30} expected actual  diff", "BPF program");
    for program in programs.iter() {
        let program_id = solana_sdk::pubkey::new_rand();
        let key = solana_sdk::pubkey::new_rand();
        let mut account = RefCell::new(AccountSharedData::default());
        let parameter_accounts = vec![KeyedAccount::new(&key, false, &mut account)];
        let count = run_program(program.0, &program_id, parameter_accounts, &[]).unwrap();
        let diff: i64 = count as i64 - program.1 as i64;
        println!(
            "  {:30} {:8} {:6} {:+5} ({:+3.0}%)",
            program.0,
            program.1,
            count,
            diff,
            100.0_f64 * count as f64 / program.1 as f64 - 100.0_f64,
        );
        if count > program.1 {
            passed = false;
        }
    }
    assert!(passed);
}
