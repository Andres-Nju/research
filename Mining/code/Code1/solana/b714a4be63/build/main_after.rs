fn main() {
    let bpf_c = !env::var("CARGO_FEATURE_BPF_C").is_err();
    if bpf_c {
        let install_dir =
            "OUT_DIR=../target/".to_string() + &env::var("PROFILE").unwrap() + &"/bpf".to_string();

        println!("cargo:warning=(not a warning) Building C-based BPF programs");
        assert!(Command::new("make")
            .current_dir("c")
            .arg("programs")
            .arg(&install_dir)
            .status()
            .expect("Failed to build C-based BPF programs")
            .success());

        rerun_if_changed(&["c/makefile"], &["c/src", "../../sdk"], &["/target/"]);
    }

    let bpf_rust = !env::var("CARGO_FEATURE_BPF_RUST").is_err();
    if bpf_rust {
        let install_dir =
            "target/".to_string() + &env::var("PROFILE").unwrap() + &"/bpf".to_string();

        assert!(Command::new("mkdir")
            .arg("-p")
            .arg(&install_dir)
            .status()
            .expect("Unable to create BPF install directory")
            .success());

        let rust_programs = [
            "128bit",
            "alloc",
            "dep_crate",
            "iter",
            "many_args",
            "external_spend",
            "noop",
            "panic",
            "param_passing",
            "sysval",
        ];
        for program in rust_programs.iter() {
            println!(
                "cargo:warning=(not a warning) Building Rust-based BPF programs: solana_bpf_rust_{}",
                program
            );
            assert!(Command::new("bash")
                .current_dir("rust")
                .args(&["./do.sh", "build", program])
                .status()
                .expect("Error calling do.sh from build.rs")
                .success());
            let src = format!(
                "target/bpfel-unknown-unknown/release/solana_bpf_rust_{}.so",
                program,
            );
            assert!(Command::new("cp")
                .arg(&src)
                .arg(&install_dir)
                .status()
                .expect(&format!("Failed to cp {} to {}", src, install_dir))
                .success());
        }

        rerun_if_changed(&[], &["rust", "../../sdk", &install_dir], &["/target/"]);
    }
}
