    fn test_cli_parse_command() {
        let test_commands = app("test", "desc", "version");

        let pubkey = Pubkey::new_rand();
        let pubkey_string = format!("{}", pubkey);
        let witness0 = Pubkey::new_rand();
        let witness0_string = format!("{}", witness0);
        let witness1 = Pubkey::new_rand();
        let witness1_string = format!("{}", witness1);
        let dt = Utc.ymd(2018, 9, 19).and_hms(17, 30, 59);

        // Test Airdrop Subcommand
        let test_airdrop = test_commands
            .clone()
            .get_matches_from(vec!["test", "airdrop", "50", "lamports"]);
        assert_eq!(
            parse_command(&test_airdrop).unwrap(),
            CliCommandInfo {
                command: CliCommand::Airdrop {
                    faucet_host: None,
                    faucet_port: solana_faucet::faucet::FAUCET_PORT,
                    lamports: 50,
                    use_lamports_unit: true,
                },
                require_keypair: true,
            }
        );

        // Test Balance Subcommand, incl pubkey and keypair-file inputs
        let keypair_file = make_tmp_path("keypair_file");
        write_keypair_file(&Keypair::new(), &keypair_file).unwrap();
        let keypair = read_keypair_file(&keypair_file).unwrap();
        let test_balance = test_commands.clone().get_matches_from(vec![
            "test",
            "balance",
            &keypair.pubkey().to_string(),
        ]);
        assert_eq!(
            parse_command(&test_balance).unwrap(),
            CliCommandInfo {
                command: CliCommand::Balance {
                    pubkey: Some(keypair.pubkey()),
                    use_lamports_unit: false
                },
                require_keypair: false
            }
        );
        let test_balance = test_commands.clone().get_matches_from(vec![
            "test",
            "balance",
            &keypair_file,
            "--lamports",
        ]);
        assert_eq!(
            parse_command(&test_balance).unwrap(),
            CliCommandInfo {
                command: CliCommand::Balance {
                    pubkey: Some(keypair.pubkey()),
                    use_lamports_unit: true
                },
                require_keypair: false
            }
        );
        let test_balance =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "balance", "--lamports"]);
        assert_eq!(
            parse_command(&test_balance).unwrap(),
            CliCommandInfo {
                command: CliCommand::Balance {
                    pubkey: None,
                    use_lamports_unit: true
                },
                require_keypair: true
            }
        );

        // Test Cancel Subcommand
        let test_cancel =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "cancel", &pubkey_string]);
        assert_eq!(
            parse_command(&test_cancel).unwrap(),
            CliCommandInfo {
                command: CliCommand::Cancel(pubkey),
                require_keypair: true
            }
        );

        // Test Confirm Subcommand
        let signature = Signature::new(&vec![1; 64]);
        let signature_string = format!("{:?}", signature);
        let test_confirm =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "confirm", &signature_string]);
        assert_eq!(
            parse_command(&test_confirm).unwrap(),
            CliCommandInfo {
                command: CliCommand::Confirm(signature),
                require_keypair: false
            }
        );
        let test_bad_signature = test_commands
            .clone()
            .get_matches_from(vec!["test", "confirm", "deadbeef"]);
        assert!(parse_command(&test_bad_signature).is_err());

        // Test CreateAddressWithSeed
        let from_pubkey = Some(Pubkey::new_rand());
        let from_str = from_pubkey.unwrap().to_string();
        for (name, program_id) in &[
            ("STAKE", solana_stake_program::id()),
            ("VOTE", solana_vote_program::id()),
            ("NONCE", solana_sdk::nonce_program::id()),
            ("STORAGE", solana_storage_program::id()),
        ] {
            let test_create_address_with_seed = test_commands.clone().get_matches_from(vec![
                "test",
                "create-address-with-seed",
                "seed",
                name,
                "--from",
                &from_str,
            ]);
            assert_eq!(
                parse_command(&test_create_address_with_seed).unwrap(),
                CliCommandInfo {
                    command: CliCommand::CreateAddressWithSeed {
                        from_pubkey,
                        seed: "seed".to_string(),
                        program_id: *program_id
                    },
                    require_keypair: false
                }
            );
        }
        let test_create_address_with_seed = test_commands.clone().get_matches_from(vec![
            "test",
            "create-address-with-seed",
            "seed",
            "STAKE",
        ]);
        assert_eq!(
            parse_command(&test_create_address_with_seed).unwrap(),
            CliCommandInfo {
                command: CliCommand::CreateAddressWithSeed {
                    from_pubkey: None,
                    seed: "seed".to_string(),
                    program_id: solana_stake_program::id(),
                },
                require_keypair: true
            }
        );

        // Test Deploy Subcommand
        let test_deploy =
            test_commands
                .clone()
                .get_matches_from(vec!["test", "deploy", "/Users/test/program.o"]);
        assert_eq!(
            parse_command(&test_deploy).unwrap(),
            CliCommandInfo {
                command: CliCommand::Deploy("/Users/test/program.o".to_string()),
                require_keypair: true
            }
        );

        // Test Simple Pay Subcommand
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
        ]);
        assert_eq!(
            parse_command(&test_pay).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: None,
                    timestamp_pubkey: None,
                    witnesses: None,
                    cancelable: false,
                    sign_only: false,
                    signers: None,
                    blockhash: None,
                },
                require_keypair: true
            }
        );

        // Test Pay Subcommand w/ Witness
        let test_pay_multiple_witnesses = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
            "--require-signature-from",
            &witness0_string,
            "--require-signature-from",
            &witness1_string,
        ]);
        assert_eq!(
            parse_command(&test_pay_multiple_witnesses).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: None,
                    timestamp_pubkey: None,
                    witnesses: Some(vec![witness0, witness1]),
                    cancelable: false,
                    sign_only: false,
                    signers: None,
                    blockhash: None,
                },
                require_keypair: true
            }
        );
        let test_pay_single_witness = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
            "--require-signature-from",
            &witness0_string,
        ]);
        assert_eq!(
            parse_command(&test_pay_single_witness).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: None,
                    timestamp_pubkey: None,
                    witnesses: Some(vec![witness0]),
                    cancelable: false,
                    sign_only: false,
                    signers: None,
                    blockhash: None,
                },
                require_keypair: true
            }
        );

        // Test Pay Subcommand w/ Timestamp
        let test_pay_timestamp = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
            "--after",
            "2018-09-19T17:30:59",
            "--require-timestamp-from",
            &witness0_string,
        ]);
        assert_eq!(
            parse_command(&test_pay_timestamp).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: Some(dt),
                    timestamp_pubkey: Some(witness0),
                    witnesses: None,
                    cancelable: false,
                    sign_only: false,
                    signers: None,
                    blockhash: None,
                },
                require_keypair: true
            }
        );

        // Test Pay Subcommand w/ sign-only
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
            "--sign-only",
        ]);
        assert_eq!(
            parse_command(&test_pay).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: None,
                    timestamp_pubkey: None,
                    witnesses: None,
                    cancelable: false,
                    sign_only: true,
                    signers: None,
                    blockhash: None,
                },
                require_keypair: true,
            }
        );

        // Test Pay Subcommand w/ signer
        let key1 = Pubkey::new_rand();
        let sig1 = Keypair::new().sign_message(&[0u8]);
        let signer1 = format!("{}={}", key1, sig1);
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
            "--signer",
            &signer1,
        ]);
        assert_eq!(
            parse_command(&test_pay).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: None,
                    timestamp_pubkey: None,
                    witnesses: None,
                    cancelable: false,
                    sign_only: false,
                    signers: Some(vec![(key1, sig1)]),
                    blockhash: None,
                },
                require_keypair: true
            }
        );

        // Test Pay Subcommand w/ signers
        let key2 = Pubkey::new_rand();
        let sig2 = Keypair::new().sign_message(&[1u8]);
        let signer2 = format!("{}={}", key2, sig2);
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
            "--signer",
            &signer1,
            "--signer",
            &signer2,
        ]);
        assert_eq!(
            parse_command(&test_pay).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: None,
                    timestamp_pubkey: None,
                    witnesses: None,
                    cancelable: false,
                    sign_only: false,
                    signers: Some(vec![(key1, sig1), (key2, sig2)]),
                    blockhash: None,
                },
                require_keypair: true
            }
        );

        // Test Pay Subcommand w/ Blockhash
        let blockhash = Hash::default();
        let blockhash_string = format!("{}", blockhash);
        let test_pay = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
            "--blockhash",
            &blockhash_string,
        ]);
        assert_eq!(
            parse_command(&test_pay).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: None,
                    timestamp_pubkey: None,
                    witnesses: None,
                    cancelable: false,
                    sign_only: false,
                    signers: None,
                    blockhash: Some(blockhash),
                },
                require_keypair: true
            }
        );

        // Test Send-Signature Subcommand
        let test_send_signature = test_commands.clone().get_matches_from(vec![
            "test",
            "send-signature",
            &pubkey_string,
            &pubkey_string,
        ]);
        assert_eq!(
            parse_command(&test_send_signature).unwrap(),
            CliCommandInfo {
                command: CliCommand::Witness(pubkey, pubkey),
                require_keypair: true
            }
        );
        let test_pay_multiple_witnesses = test_commands.clone().get_matches_from(vec![
            "test",
            "pay",
            &pubkey_string,
            "50",
            "lamports",
            "--after",
            "2018-09-19T17:30:59",
            "--require-signature-from",
            &witness0_string,
            "--require-timestamp-from",
            &witness0_string,
            "--require-signature-from",
            &witness1_string,
        ]);
        assert_eq!(
            parse_command(&test_pay_multiple_witnesses).unwrap(),
            CliCommandInfo {
                command: CliCommand::Pay {
                    lamports: 50,
                    to: pubkey,
                    timestamp: Some(dt),
                    timestamp_pubkey: Some(witness0),
                    witnesses: Some(vec![witness0, witness1]),
                    cancelable: false,
                    sign_only: false,
                    signers: None,
                    blockhash: None,
                },
                require_keypair: true
            }
        );

        // Test Send-Timestamp Subcommand
        let test_send_timestamp = test_commands.clone().get_matches_from(vec![
            "test",
            "send-timestamp",
            &pubkey_string,
            &pubkey_string,
            "--date",
            "2018-09-19T17:30:59",
        ]);
        assert_eq!(
            parse_command(&test_send_timestamp).unwrap(),
            CliCommandInfo {
                command: CliCommand::TimeElapsed(pubkey, pubkey, dt),
                require_keypair: true
            }
        );
        let test_bad_timestamp = test_commands.clone().get_matches_from(vec![
            "test",
            "send-timestamp",
            &pubkey_string,
            &pubkey_string,
            "--date",
            "20180919T17:30:59",
        ]);
        assert!(parse_command(&test_bad_timestamp).is_err());
    }
