fn build_sbf_package(config: &Config, target_directory: &Path, package: &cargo_metadata::Package) {
    let program_name = {
        let cdylib_targets = package
            .targets
            .iter()
            .filter_map(|target| {
                if target.crate_types.contains(&"cdylib".to_string()) {
                    Some(&target.name)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match cdylib_targets.len() {
            0 => {
                warn!(
                    "Note: {} crate does not contain a cdylib target",
                    package.name
                );
                None
            }
            1 => Some(cdylib_targets[0].replace('-', "_")),
            _ => {
                error!(
                    "{} crate contains multiple cdylib targets: {:?}",
                    package.name, cdylib_targets
                );
                exit(1);
            }
        }
    };

    let legacy_program_feature_present = package.name == "solana-sdk";
    let root_package_dir = &package.manifest_path.parent().unwrap_or_else(|| {
        error!("Unable to get directory of {}", package.manifest_path);
        exit(1);
    });

    let sbf_out_dir = config
        .sbf_out_dir
        .as_ref()
        .cloned()
        .unwrap_or_else(|| target_directory.join("deploy"));

    let target_build_directory = target_directory.join("sbf-solana-solana").join("release");

    env::set_current_dir(&root_package_dir).unwrap_or_else(|err| {
        error!(
            "Unable to set current directory to {}: {}",
            root_package_dir, err
        );
        exit(1);
    });

    info!("SBF SDK: {}", config.sbf_sdk.display());
    if config.no_default_features {
        info!("No default features");
    }
    if !config.features.is_empty() {
        info!("Features: {}", config.features.join(" "));
    }
    if legacy_program_feature_present {
        info!("Legacy program feature detected");
    }
    let sbf_tools_download_file_name = if cfg!(target_os = "windows") {
        "solana-sbf-tools-windows.tar.bz2"
    } else if cfg!(target_os = "macos") {
        "solana-sbf-tools-osx.tar.bz2"
    } else {
        "solana-sbf-tools-linux.tar.bz2"
    };

    let home_dir = PathBuf::from(env::var("HOME").unwrap_or_else(|err| {
        error!("Can't get home directory path: {}", err);
        exit(1);
    }));
    let package = "sbf-tools";
    let target_path = home_dir
        .join(".cache")
        .join("solana")
        .join(config.sbf_tools_version)
        .join(package);
    install_if_missing(
        config,
        package,
        "https://github.com/solana-labs/bpf-tools/releases/download",
        sbf_tools_download_file_name,
        &target_path,
    )
    .unwrap_or_else(|err| {
        // The package version directory doesn't contain a valid
        // installation, and it should be removed.
        let target_path_parent = target_path.parent().expect("Invalid package path");
        fs::remove_dir_all(&target_path_parent).unwrap_or_else(|err| {
            error!(
                "Failed to remove {} while recovering from installation failure: {}",
                target_path_parent.to_string_lossy(),
                err,
            );
            exit(1);
        });
        error!("Failed to install sbf-tools: {}", err);
        exit(1);
    });
    link_sbf_toolchain(config);

    let llvm_bin = config
        .sbf_sdk
        .join("dependencies")
        .join("sbf-tools")
        .join("llvm")
        .join("bin");
    env::set_var("CC", llvm_bin.join("clang"));
    env::set_var("AR", llvm_bin.join("llvm-ar"));
    env::set_var("OBJDUMP", llvm_bin.join("llvm-objdump"));
    env::set_var("OBJCOPY", llvm_bin.join("llvm-objcopy"));

    let rustflags = env::var("RUSTFLAGS").ok();
    let rustflags = rustflags.as_deref().unwrap_or_default();
    if config.remap_cwd {
        let rustflags = format!("{} -Zremap-cwd-prefix=", &rustflags);
        env::set_var("RUSTFLAGS", &rustflags);
    }
    if config.verbose {
        debug!(
            "RUSTFLAGS=\"{}\"",
            env::var("RUSTFLAGS").ok().unwrap_or_default()
        );
    }

    // RUSTC variable overrides cargo +<toolchain> mechanism of
    // selecting the rust compiler and makes cargo run a rust compiler
    // other than the one linked in BPF toolchain. We have to prevent
    // this by removing RUSTC from the child process environment.
    if env::var("RUSTC").is_ok() {
        warn!(
            "Removed RUSTC from cargo environment, because it overrides +sbf cargo command line option."
        );
        env::remove_var("RUSTC")
    }

    let cargo_build = PathBuf::from("cargo");
    let mut cargo_build_args = vec![
        "+sbf",
        "build",
        "--target",
        "sbf-solana-solana",
        "--release",
    ];
    if config.no_default_features {
        cargo_build_args.push("--no-default-features");
    }
    for feature in &config.features {
        cargo_build_args.push("--features");
        cargo_build_args.push(feature);
    }
    if legacy_program_feature_present {
        if !config.no_default_features {
            cargo_build_args.push("--no-default-features");
        }
        cargo_build_args.push("--features=program");
    }
    if config.verbose {
        cargo_build_args.push("--verbose");
    }
    if let Some(jobs) = &config.jobs {
        cargo_build_args.push("--jobs");
        cargo_build_args.push(jobs);
    }
    if let Some(args) = &config.cargo_args {
        for arg in args {
            cargo_build_args.push(arg);
        }
    }
    let output = spawn(
        &cargo_build,
        &cargo_build_args,
        config.generate_child_script_on_failure,
    );
    if config.verbose {
        debug!("{}", output);
    }

    if let Some(program_name) = program_name {
        let program_unstripped_so = target_build_directory.join(&format!("{}.so", program_name));
        let program_dump = sbf_out_dir.join(&format!("{}-dump.txt", program_name));
        let program_so = sbf_out_dir.join(&format!("{}.so", program_name));
        let program_keypair = sbf_out_dir.join(&format!("{}-keypair.json", program_name));

        fn file_older_or_missing(prerequisite_file: &Path, target_file: &Path) -> bool {
            let prerequisite_metadata = fs::metadata(prerequisite_file).unwrap_or_else(|err| {
                error!(
                    "Unable to get file metadata for {}: {}",
                    prerequisite_file.display(),
                    err
                );
                exit(1);
            });

            if let Ok(target_metadata) = fs::metadata(target_file) {
                use std::time::UNIX_EPOCH;
                prerequisite_metadata.modified().unwrap_or(UNIX_EPOCH)
                    > target_metadata.modified().unwrap_or(UNIX_EPOCH)
            } else {
                true
            }
        }

        if !program_keypair.exists() {
            write_keypair_file(&Keypair::new(), &program_keypair).unwrap_or_else(|err| {
                error!(
                    "Unable to get create {}: {}",
                    program_keypair.display(),
                    err
                );
                exit(1);
            });
        }

        if file_older_or_missing(&program_unstripped_so, &program_so) {
            #[cfg(windows)]
            let output = spawn(
                &llvm_bin.join("llvm-objcopy"),
                &[
                    "--strip-all".as_ref(),
                    program_unstripped_so.as_os_str(),
                    program_so.as_os_str(),
                ],
                config.generate_child_script_on_failure,
            );
            #[cfg(not(windows))]
            let output = spawn(
                &config.sbf_sdk.join("scripts").join("strip.sh"),
                &[&program_unstripped_so, &program_so],
                config.generate_child_script_on_failure,
            );
            if config.verbose {
                debug!("{}", output);
            }
        }

        if config.dump && file_older_or_missing(&program_unstripped_so, &program_dump) {
            let dump_script = config.sbf_sdk.join("scripts").join("dump.sh");
            #[cfg(windows)]
            {
                error!("Using Bash scripts from within a program is not supported on Windows, skipping `--dump`.");
                error!(
                    "Please run \"{} {} {}\" from a Bash-supporting shell, then re-run this command to see the processed program dump.",
                    &dump_script.display(),
                    &program_unstripped_so.display(),
                    &program_dump.display());
            }
            #[cfg(not(windows))]
            {
                let output = spawn(
                    &dump_script,
                    &[&program_unstripped_so, &program_dump],
                    config.generate_child_script_on_failure,
                );
                if config.verbose {
                    debug!("{}", output);
                }
            }
            postprocess_dump(&program_dump);
        }

        check_undefined_symbols(config, &program_so);

        info!("To deploy this program:");
        info!("  $ solana program deploy {}", program_so.display());
        info!("The program address will default to this keypair (override with --program-id):");
        info!("  {}", program_keypair.display());
    } else if config.dump {
        warn!("Note: --dump is only available for crates with a cdylib target");
    }
}
