fn do_main(matches: &ArgMatches<'_>) -> Result<(), Box<dyn error::Error>> {
    let config = if let Some(config_file) = matches.value_of("config_file") {
        Config::load(config_file).unwrap_or_default()
    } else {
        Config::default()
    };

    let wallet_manager = if check_for_usb(std::env::args()) {
        maybe_wallet_manager()?
    } else {
        None
    };

    match matches.subcommand() {
        ("pubkey", Some(matches)) => {
            let pubkey = get_keypair_from_matches(matches, config, wallet_manager)?.try_pubkey()?;

            if matches.is_present("outfile") {
                let outfile = matches.value_of("outfile").unwrap();
                check_for_overwrite(&outfile, &matches);
                write_pubkey_file(outfile, pubkey)?;
            } else {
                println!("{}", pubkey);
            }
        }
        ("new", Some(matches)) => {
            let mut path = dirs::home_dir().expect("home directory");
            let outfile = if matches.is_present("outfile") {
                matches.value_of("outfile")
            } else if matches.is_present("no_outfile") {
                None
            } else {
                path.extend(&[".config", "solana", "id.json"]);
                Some(path.to_str().unwrap())
            };

            match outfile {
                Some("-") => (),
                Some(outfile) => check_for_overwrite(&outfile, &matches),
                None => (),
            }

            let word_count = value_t!(matches.value_of("word_count"), usize).unwrap();
            let mnemonic_type = MnemonicType::for_word_count(word_count)?;
            let mnemonic = Mnemonic::new(mnemonic_type, Language::English);
            let passphrase = if matches.is_present("no_passphrase") {
                NO_PASSPHRASE.to_string()
            } else {
                eprintln!("Generating a new keypair");
                prompt_passphrase(
                    "For added security, enter a passphrase (empty for no passphrase): ",
                )?
            };
            let seed = Seed::new(&mnemonic, &passphrase);
            let keypair = keypair_from_seed(seed.as_bytes())?;

            if let Some(outfile) = outfile {
                output_keypair(&keypair, &outfile, "new")
                    .map_err(|err| format!("Unable to write {}: {}", outfile, err))?;
            }

            let silent = matches.is_present("silent");
            if !silent {
                let phrase: &str = mnemonic.phrase();
                let divider = String::from_utf8(vec![b'='; phrase.len()]).unwrap();
                eprintln!(
                    "{}\npubkey: {}\n{}\nSave this seed phrase to recover your new keypair:\n{}\n{}",
                    &divider, keypair.pubkey(), &divider, phrase, &divider
                );
            }
        }
        ("recover", Some(matches)) => {
            let mut path = dirs::home_dir().expect("home directory");
            let outfile = if matches.is_present("outfile") {
                matches.value_of("outfile").unwrap()
            } else {
                path.extend(&[".config", "solana", "id.json"]);
                path.to_str().unwrap()
            };

            if outfile != "-" {
                check_for_overwrite(&outfile, &matches);
            }

            let skip_validation = matches.is_present(SKIP_SEED_PHRASE_VALIDATION_ARG.name);
            let keypair = keypair_from_seed_phrase("recover", skip_validation, true)?;
            output_keypair(&keypair, &outfile, "recovered")?;
