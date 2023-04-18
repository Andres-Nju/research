    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid")
    }
}

impl error::Error for CliError {
    fn description(&self) -> &str {
        "invalid"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

pub struct CliConfig {
    pub command: CliCommand,
    pub json_rpc_url: String,
    pub keypair: Keypair,
    pub keypair_path: Option<String>,
    pub rpc_client: Option<RpcClient>,
    pub verbose: bool,
}

impl CliConfig {
    pub fn default_keypair_path() -> String {
        let mut keypair_path = dirs::home_dir().expect("home directory");
        keypair_path.extend(&[".config", "solana", "id.json"]);
        keypair_path.to_str().unwrap().to_string()
    }

    pub fn default_json_rpc_url() -> String {
        "http://127.0.0.1:8899".to_string()
    }
}

impl Default for CliConfig {
    fn default() -> CliConfig {
        CliConfig {
            command: CliCommand::Balance {
                pubkey: Some(Pubkey::default()),
                use_lamports_unit: false,
            },
            json_rpc_url: Self::default_json_rpc_url(),
            keypair: Keypair::new(),
            keypair_path: Some(Self::default_keypair_path()),
            rpc_client: None,
            verbose: false,
        }
    }
}

pub fn parse_command(matches: &ArgMatches<'_>) -> Result<CliCommandInfo, Box<dyn error::Error>> {
    let response = match matches.subcommand() {
        // Cluster Query Commands
        ("catchup", Some(matches)) => parse_catchup(matches),
        ("cluster-version", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::ClusterVersion,
            require_keypair: false,
        }),
        ("create-address-with-seed", Some(matches)) => parse_create_address_with_seed(matches),
        ("fees", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::Fees,
            require_keypair: false,
        }),
        ("get-block-time", Some(matches)) => parse_get_block_time(matches),
        ("get-epoch-info", Some(matches)) => parse_get_epoch_info(matches),
        ("get-genesis-hash", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::GetGenesisHash,
            require_keypair: false,
        }),
        ("get-slot", Some(matches)) => parse_get_slot(matches),
        ("get-transaction-count", Some(matches)) => parse_get_transaction_count(matches),
        ("ping", Some(matches)) => parse_cluster_ping(matches),
        ("show-block-production", Some(matches)) => parse_show_block_production(matches),
        ("show-gossip", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::ShowGossip,
            require_keypair: false,
        }),
        ("show-validators", Some(matches)) => parse_show_validators(matches),
        // Nonce Commands
        ("authorize-nonce-account", Some(matches)) => parse_authorize_nonce_account(matches),
        ("create-nonce-account", Some(matches)) => parse_nonce_create_account(matches),
        ("get-nonce", Some(matches)) => parse_get_nonce(matches),
        ("new-nonce", Some(matches)) => parse_new_nonce(matches),
        ("show-nonce-account", Some(matches)) => parse_show_nonce_account(matches),
        ("withdraw-from-nonce-account", Some(matches)) => {
            parse_withdraw_from_nonce_account(matches)
        }
        // Program Deployment
        ("deploy", Some(matches)) => Ok(CliCommandInfo {
            command: CliCommand::Deploy(matches.value_of("program_location").unwrap().to_string()),
            require_keypair: true,
        }),
        // Stake Commands
        ("create-stake-account", Some(matches)) => parse_stake_create_account(matches),
        ("delegate-stake", Some(matches)) => parse_stake_delegate_stake(matches),
        ("withdraw-stake", Some(matches)) => parse_stake_withdraw_stake(matches),
        ("deactivate-stake", Some(matches)) => parse_stake_deactivate_stake(matches),
        ("stake-authorize-staker", Some(matches)) => {
            parse_stake_authorize(matches, StakeAuthorize::Staker)
        }
        ("stake-authorize-withdrawer", Some(matches)) => {
            parse_stake_authorize(matches, StakeAuthorize::Withdrawer)
        }
        ("redeem-vote-credits", Some(matches)) => parse_redeem_vote_credits(matches),
        ("show-stake-account", Some(matches)) => parse_show_stake_account(matches),
        ("show-stake-history", Some(matches)) => parse_show_stake_history(matches),
        // Storage Commands
        ("create-archiver-storage-account", Some(matches)) => {
            parse_storage_create_archiver_account(matches)
        }
        ("create-validator-storage-account", Some(matches)) => {
            parse_storage_create_validator_account(matches)
        }
        ("claim-storage-reward", Some(matches)) => parse_storage_claim_reward(matches),
        ("show-storage-account", Some(matches)) => parse_storage_get_account_command(matches),
        // Validator Info Commands
        ("validator-info", Some(matches)) => match matches.subcommand() {
            ("publish", Some(matches)) => parse_validator_info_command(matches),
            ("get", Some(matches)) => parse_get_validator_info_command(matches),
            ("", None) => {
                eprintln!("{}", matches.usage());
                Err(CliError::CommandNotRecognized(
                    "no validator-info subcommand given".to_string(),
                ))
            }
            _ => unreachable!(),
        },
        // Vote Commands
        ("create-vote-account", Some(matches)) => parse_vote_create_account(matches),
        ("vote-update-validator", Some(matches)) => parse_vote_update_validator(matches),
        ("vote-authorize-voter", Some(matches)) => {
            parse_vote_authorize(matches, VoteAuthorize::Voter)
        }
        ("vote-authorize-withdrawer", Some(matches)) => {
            parse_vote_authorize(matches, VoteAuthorize::Withdrawer)
        }
        ("show-vote-account", Some(matches)) => parse_vote_get_account_command(matches),
        ("uptime", Some(matches)) => parse_vote_uptime_command(matches),
        // Wallet Commands
        ("address", Some(_matches)) => Ok(CliCommandInfo {
            command: CliCommand::Address,
            require_keypair: true,
        }),
        ("airdrop", Some(matches)) => {
            let faucet_port = matches
                .value_of("faucet_port")
                .unwrap()
                .parse()
                .or_else(|err| {
                    Err(CliError::BadParameter(format!(
                        "Invalid faucet port: {:?}",
                        err
                    )))
                })?;

            let faucet_host = if let Some(faucet_host) = matches.value_of("faucet_host") {
                Some(solana_net_utils::parse_host(faucet_host).or_else(|err| {
                    Err(CliError::BadParameter(format!(
                        "Invalid faucet host: {:?}",
                        err
                    )))
                })?)
            } else {
                None
            };
            let lamports = required_lamports_from(matches, "amount", "unit")?;
            let use_lamports_unit = matches.value_of("unit") == Some("lamports");
            Ok(CliCommandInfo {
                command: CliCommand::Airdrop {
                    faucet_host,
                    faucet_port,
                    lamports,
                    use_lamports_unit,
                },
                require_keypair: true,
            })
        }
        ("balance", Some(matches)) => {
            let pubkey = pubkey_of(&matches, "pubkey");
            Ok(CliCommandInfo {
                command: CliCommand::Balance {
                    pubkey,
                    use_lamports_unit: matches.is_present("lamports"),
                },
                require_keypair: pubkey.is_none(),
            })
        }
        ("cancel", Some(matches)) => {
            let process_id = value_of(matches, "process_id").unwrap();
            Ok(CliCommandInfo {
                command: CliCommand::Cancel(process_id),
                require_keypair: true,
            })
        }
        ("confirm", Some(matches)) => match matches.value_of("signature").unwrap().parse() {
            Ok(signature) => Ok(CliCommandInfo {
                command: CliCommand::Confirm(signature),
                require_keypair: false,
            }),
            _ => {
                eprintln!("{}", matches.usage());
                Err(CliError::BadParameter("Invalid signature".to_string()))
            }
        },
        ("pay", Some(matches)) => {
            let lamports = required_lamports_from(matches, "amount", "unit")?;
            let to = pubkey_of(&matches, "to").unwrap();
            let timestamp = if matches.is_present("timestamp") {
                // Parse input for serde_json
                let date_string = if !matches.value_of("timestamp").unwrap().contains('Z') {
                    format!("\"{}Z\"", matches.value_of("timestamp").unwrap())
                } else {
                    format!("\"{}\"", matches.value_of("timestamp").unwrap())
                };
                Some(serde_json::from_str(&date_string)?)
            } else {
                None
            };
            let timestamp_pubkey = value_of(&matches, "timestamp_pubkey");
            let witnesses = values_of(&matches, "witness");
            let cancelable = matches.is_present("cancelable");
            let sign_only = matches.is_present("sign_only");
            let signers = pubkeys_sigs_of(&matches, "signer");
            let blockhash = value_of(&matches, "blockhash");

            Ok(CliCommandInfo {
                command: CliCommand::Pay {
                    lamports,
                    to,
                    timestamp,
                    timestamp_pubkey,
                    witnesses,
                    cancelable,
                    sign_only,
                    signers,
                    blockhash,
                },
                require_keypair: !sign_only,
            })
        }
        ("show-account", Some(matches)) => {
            let account_pubkey = pubkey_of(matches, "account_pubkey").unwrap();
            let output_file = matches.value_of("output_file");
            let use_lamports_unit = matches.is_present("lamports");
            Ok(CliCommandInfo {
                command: CliCommand::ShowAccount {
                    pubkey: account_pubkey,
                    output_file: output_file.map(ToString::to_string),
                    use_lamports_unit,
                },
                require_keypair: false,
            })
        }
        ("send-signature", Some(matches)) => {
            let to = value_of(&matches, "to").unwrap();
            let process_id = value_of(&matches, "process_id").unwrap();
            Ok(CliCommandInfo {
                command: CliCommand::Witness(to, process_id),
                require_keypair: true,
            })
        }
        ("send-timestamp", Some(matches)) => {
            let to = value_of(&matches, "to").unwrap();
            let process_id = value_of(&matches, "process_id").unwrap();
            let dt = if matches.is_present("datetime") {
                // Parse input for serde_json
                let date_string = if !matches.value_of("datetime").unwrap().contains('Z') {
                    format!("\"{}Z\"", matches.value_of("datetime").unwrap())
                } else {
                    format!("\"{}\"", matches.value_of("datetime").unwrap())
                };
                serde_json::from_str(&date_string)?
            } else {
                Utc::now()
            };
            Ok(CliCommandInfo {
                command: CliCommand::TimeElapsed(to, process_id, dt),
                require_keypair: true,
            })
        }
        //
        ("", None) => {
            eprintln!("{}", matches.usage());
            Err(CliError::CommandNotRecognized(
                "no subcommand given".to_string(),
            ))
        }
        _ => unreachable!(),
    }?;
    Ok(response)
}

pub type ProcessResult = Result<String, Box<dyn std::error::Error>>;

pub fn check_account_for_fee(
    rpc_client: &RpcClient,
    account_pubkey: &Pubkey,
    fee_calculator: &FeeCalculator,
    message: &Message,
) -> Result<(), Box<dyn error::Error>> {
    check_account_for_multiple_fees(rpc_client, account_pubkey, fee_calculator, &[message])
}

fn check_account_for_multiple_fees(
    rpc_client: &RpcClient,
    account_pubkey: &Pubkey,
    fee_calculator: &FeeCalculator,
    messages: &[&Message],
) -> Result<(), Box<dyn error::Error>> {
    let balance = rpc_client.retry_get_balance(account_pubkey, 5)?;
    if let Some(lamports) = balance {
        if lamports
            >= messages
                .iter()
                .map(|message| fee_calculator.calculate_fee(message))
                .sum()
        {
            return Ok(());
        }
    }
    Err(CliError::InsufficientFundsForFee.into())
}

pub fn check_unique_pubkeys(
    pubkey0: (&Pubkey, String),
    pubkey1: (&Pubkey, String),
) -> Result<(), CliError> {
    if pubkey0.0 == pubkey1.0 {
        Err(CliError::BadParameter(format!(
            "Identical pubkeys found: `{}` and `{}` must be unique",
            pubkey0.1, pubkey1.1
        )))
    } else {
        Ok(())
    }
}

pub fn get_blockhash_fee_calculator(
    rpc_client: &RpcClient,
    sign_only: bool,
    blockhash: Option<Hash>,
) -> Result<(Hash, FeeCalculator), Box<dyn std::error::Error>> {
    Ok(if let Some(blockhash) = blockhash {
        if sign_only {
            (blockhash, FeeCalculator::default())
        } else {
            (blockhash, rpc_client.get_recent_blockhash()?.1)
        }
    } else {
        rpc_client.get_recent_blockhash()?
    })
}

pub fn return_signers(tx: &Transaction) -> ProcessResult {
    println_signers(tx);
    let signers: Vec<_> = tx
        .signatures
        .iter()
        .zip(tx.message.account_keys.clone())
        .map(|(signature, pubkey)| format!("{}={}", pubkey, signature))
        .collect();

    Ok(json!({
        "blockhash": tx.message.recent_blockhash.to_string(),
        "signers": &signers,
    })
    .to_string())
}

pub fn replace_signatures(tx: &mut Transaction, signers: &[(Pubkey, Signature)]) -> ProcessResult {
    tx.replace_signatures(signers).map_err(|_| {
        CliError::BadParameter(
            "Transaction construction failed, incorrect signature or public key provided"
                .to_string(),
        )
    })?;
    Ok("".to_string())
}

pub fn parse_create_address_with_seed(
    matches: &ArgMatches<'_>,
) -> Result<CliCommandInfo, CliError> {
    let from_pubkey = pubkey_of(matches, "from");

    let require_keypair = from_pubkey.is_none();

    let program_id = match matches.value_of("program_id").unwrap() {
        "STAKE" => solana_stake_program::id(),
        "VOTE" => solana_vote_program::id(),
        "STORAGE" => solana_storage_program::id(),
        "NONCE" => solana_sdk::nonce_program::id(),
        _ => pubkey_of(matches, "program_id").unwrap(),
    };

    let seed = matches.value_of("seed").unwrap().to_string();

    if seed.len() > MAX_ADDRESS_SEED_LEN {
        return Err(CliError::BadParameter(
            "Address seed must not be longer than 32 bytes".to_string(),
        ));
    }

    Ok(CliCommandInfo {
        command: CliCommand::CreateAddressWithSeed {
            from_pubkey,
            seed,
            program_id,
        },
        require_keypair,
    })
}

fn process_create_address_with_seed(
    config: &CliConfig,
    from_pubkey: Option<&Pubkey>,
    seed: &str,
    program_id: &Pubkey,
) -> ProcessResult {
    let config_pubkey = config.keypair.pubkey();
    let from_pubkey = from_pubkey.unwrap_or(&config_pubkey);
    let address = create_address_with_seed(from_pubkey, seed, program_id)?;
    Ok(address.to_string())
}

fn process_airdrop(
    rpc_client: &RpcClient,
    config: &CliConfig,
    faucet_addr: &SocketAddr,
    lamports: u64,
    use_lamports_unit: bool,
) -> ProcessResult {
    println!(
        "Requesting airdrop of {} from {}",
        build_balance_message(lamports, use_lamports_unit, true),
        faucet_addr
    );
    let previous_balance = match rpc_client.retry_get_balance(&config.keypair.pubkey(), 5)? {
        Some(lamports) => lamports,
        None => {
            return Err(CliError::RpcRequestError(
                "Received result of an unexpected type".to_string(),
            )
            .into())
        }
    };

    request_and_confirm_airdrop(&rpc_client, faucet_addr, &config.keypair.pubkey(), lamports)?;

    let current_balance = rpc_client
        .retry_get_balance(&config.keypair.pubkey(), 5)?
        .unwrap_or(previous_balance);

    Ok(build_balance_message(
        current_balance,
        use_lamports_unit,
        true,
    ))
}

fn process_balance(
    rpc_client: &RpcClient,
    config: &CliConfig,
    pubkey: &Option<Pubkey>,
    use_lamports_unit: bool,
) -> ProcessResult {
    let pubkey = pubkey.unwrap_or(config.keypair.pubkey());
    let balance = rpc_client.retry_get_balance(&pubkey, 5)?;
    match balance {
        Some(lamports) => Ok(build_balance_message(lamports, use_lamports_unit, true)),
        None => Err(
            CliError::RpcRequestError("Received result of an unexpected type".to_string()).into(),
        ),
    }
}

fn process_confirm(rpc_client: &RpcClient, signature: &Signature) -> ProcessResult {
    match rpc_client.get_signature_status(&signature.to_string()) {
        Ok(status) => {
            if let Some(result) = status {
                match result {
                    Ok(_) => Ok("Confirmed".to_string()),
                    Err(err) => Ok(format!("Transaction failed with error {:?}", err)),
                }
            } else {
                Ok("Not found".to_string())
            }
        }
        Err(err) => Err(CliError::RpcRequestError(format!("Unable to confirm: {:?}", err)).into()),
    }
}

fn process_show_account(
    rpc_client: &RpcClient,
    _config: &CliConfig,
    account_pubkey: &Pubkey,
    output_file: &Option<String>,
    use_lamports_unit: bool,
) -> ProcessResult {
    let account = rpc_client.get_account(account_pubkey)?;

    println!();
    println_name_value("Public Key:", &account_pubkey.to_string());
    println_name_value(
        "Balance:",
        &build_balance_message(account.lamports, use_lamports_unit, true),
    );
    println_name_value("Owner:", &account.owner.to_string());
    println_name_value("Executable:", &account.executable.to_string());

    if let Some(output_file) = output_file {
        let mut f = File::create(output_file)?;
        f.write_all(&account.data)?;
        println!();
        println!("Wrote account data to {}", output_file);
    } else if !account.data.is_empty() {
        use pretty_hex::*;
        println!("{:?}", account.data.hex_dump());
    }

    Ok("".to_string())
}

fn process_deploy(
    rpc_client: &RpcClient,
    config: &CliConfig,
    program_location: &str,
) -> ProcessResult {
    let program_id = Keypair::new();
    let mut file = File::open(program_location).map_err(|err| {
        CliError::DynamicProgramError(format!("Unable to open program file: {}", err).to_string())
    })?;
    let mut program_data = Vec::new();
    file.read_to_end(&mut program_data).map_err(|err| {
        CliError::DynamicProgramError(format!("Unable to read program file: {}", err).to_string())
    })?;

    // Build transactions to calculate fees
    let mut messages: Vec<&Message> = Vec::new();
    let (blockhash, fee_calculator) = rpc_client.get_recent_blockhash()?;
    let minimum_balance = rpc_client.get_minimum_balance_for_rent_exemption(program_data.len())?;
    let mut create_account_tx = system_transaction::create_account(
        &config.keypair,
        &program_id,
        blockhash,
        minimum_balance.max(1),
        program_data.len() as u64,
        &bpf_loader::id(),
    );
    messages.push(&create_account_tx.message);
    let signers = [&config.keypair, &program_id];
    let write_transactions: Vec<_> = program_data
        .chunks(USERDATA_CHUNK_SIZE)
        .zip(0..)
        .map(|(chunk, i)| {
            let instruction = loader_instruction::write(
                &program_id.pubkey(),
                &bpf_loader::id(),
                (i * USERDATA_CHUNK_SIZE) as u32,
                chunk.to_vec(),
            );
            let message = Message::new_with_payer(vec![instruction], Some(&signers[0].pubkey()));
            Transaction::new(&signers, message, blockhash)
        })
        .collect();
    for transaction in write_transactions.iter() {
        messages.push(&transaction.message);
    }

    let instruction = loader_instruction::finalize(&program_id.pubkey(), &bpf_loader::id());
    let message = Message::new_with_payer(vec![instruction], Some(&signers[0].pubkey()));
    let mut finalize_tx = Transaction::new(&signers, message, blockhash);
    messages.push(&finalize_tx.message);

    check_account_for_multiple_fees(
        rpc_client,
        &config.keypair.pubkey(),
        &fee_calculator,
        &messages,
    )?;

    trace!("Creating program account");
    let result = rpc_client.send_and_confirm_transaction(&mut create_account_tx, &signers);
    log_instruction_custom_error::<SystemError>(result)
        .map_err(|_| CliError::DynamicProgramError("Program allocate space failed".to_string()))?;

    trace!("Writing program data");
    rpc_client.send_and_confirm_transactions(write_transactions, &signers)?;

    trace!("Finalizing program account");
    rpc_client
        .send_and_confirm_transaction(&mut finalize_tx, &signers)
        .map_err(|_| {
            CliError::DynamicProgramError("Program finalize transaction failed".to_string())
        })?;

    Ok(json!({
        "programId": format!("{}", program_id.pubkey()),
    })
    .to_string())
}

#[allow(clippy::too_many_arguments)]
fn process_pay(
    rpc_client: &RpcClient,
    config: &CliConfig,
    lamports: u64,
    to: &Pubkey,
    timestamp: Option<DateTime<Utc>>,
    timestamp_pubkey: Option<Pubkey>,
    witnesses: &Option<Vec<Pubkey>>,
    cancelable: bool,
    sign_only: bool,
    signers: &Option<Vec<(Pubkey, Signature)>>,
    blockhash: Option<Hash>,
) -> ProcessResult {
    check_unique_pubkeys(
        (&config.keypair.pubkey(), "cli keypair".to_string()),
        (to, "to".to_string()),
    )?;

    let (blockhash, fee_calculator) =
        get_blockhash_fee_calculator(rpc_client, sign_only, blockhash)?;

    let cancelable = if cancelable {
        Some(config.keypair.pubkey())
    } else {
        None
    };

    if timestamp == None && *witnesses == None {
        let mut tx = system_transaction::transfer(&config.keypair, to, lamports, blockhash);
        if let Some(signers) = signers {
            replace_signatures(&mut tx, &signers)?;
        }

        if sign_only {
            return_signers(&tx)
        } else {
            check_account_for_fee(
                rpc_client,
                &config.keypair.pubkey(),
                &fee_calculator,
                &tx.message,
            )?;
            let result = rpc_client.send_and_confirm_transaction(&mut tx, &[&config.keypair]);
            log_instruction_custom_error::<SystemError>(result)
        }
    } else if *witnesses == None {
        let dt = timestamp.unwrap();
        let dt_pubkey = match timestamp_pubkey {
            Some(pubkey) => pubkey,
            None => config.keypair.pubkey(),
        };

        let contract_state = Keypair::new();

        // Initializing contract
        let ixs = budget_instruction::on_date(
            &config.keypair.pubkey(),
            to,
            &contract_state.pubkey(),
            dt,
            &dt_pubkey,
            cancelable,
            lamports,
        );
        let mut tx = Transaction::new_signed_instructions(
            &[&config.keypair, &contract_state],
            ixs,
            blockhash,
        );
        if let Some(signers) = signers {
            replace_signatures(&mut tx, &signers)?;
        }
        if sign_only {
            return_signers(&tx)
        } else {
            check_account_for_fee(
                rpc_client,
                &config.keypair.pubkey(),
                &fee_calculator,
                &tx.message,
            )?;
            let result = rpc_client
                .send_and_confirm_transaction(&mut tx, &[&config.keypair, &contract_state]);
            let signature_str = log_instruction_custom_error::<BudgetError>(result)?;

            Ok(json!({
                "signature": signature_str,
                "processId": format!("{}", contract_state.pubkey()),
            })
            .to_string())
        }
    } else if timestamp == None {
        let witness = if let Some(ref witness_vec) = *witnesses {
            witness_vec[0]
        } else {
            return Err(CliError::BadParameter(
                "Could not parse required signature pubkey(s)".to_string(),
            )
            .into());
        };

        let contract_state = Keypair::new();

        // Initializing contract
        let ixs = budget_instruction::when_signed(
            &config.keypair.pubkey(),
            to,
            &contract_state.pubkey(),
            &witness,
            cancelable,
            lamports,
        );
        let mut tx = Transaction::new_signed_instructions(
            &[&config.keypair, &contract_state],
            ixs,
            blockhash,
        );
        if let Some(signers) = signers {
            replace_signatures(&mut tx, &signers)?;
        }
        if sign_only {
            return_signers(&tx)
        } else {
            let result = rpc_client
                .send_and_confirm_transaction(&mut tx, &[&config.keypair, &contract_state]);
            check_account_for_fee(
                rpc_client,
                &config.keypair.pubkey(),
                &fee_calculator,
                &tx.message,
            )?;
            let signature_str = log_instruction_custom_error::<BudgetError>(result)?;

            Ok(json!({
                "signature": signature_str,
                "processId": format!("{}", contract_state.pubkey()),
            })
            .to_string())
        }
    } else {
        Ok("Combo transactions not yet handled".to_string())
    }
}

fn process_cancel(rpc_client: &RpcClient, config: &CliConfig, pubkey: &Pubkey) -> ProcessResult {
    let (blockhash, fee_calculator) = rpc_client.get_recent_blockhash()?;
    let ix = budget_instruction::apply_signature(
        &config.keypair.pubkey(),
        pubkey,
        &config.keypair.pubkey(),
    );
    let mut tx = Transaction::new_signed_instructions(&[&config.keypair], vec![ix], blockhash);
    check_account_for_fee(
        rpc_client,
        &config.keypair.pubkey(),
        &fee_calculator,
        &tx.message,
    )?;
    let result = rpc_client.send_and_confirm_transaction(&mut tx, &[&config.keypair]);
    log_instruction_custom_error::<BudgetError>(result)
}

fn process_time_elapsed(
    rpc_client: &RpcClient,
    config: &CliConfig,
    to: &Pubkey,
    pubkey: &Pubkey,
    dt: DateTime<Utc>,
) -> ProcessResult {
    let (blockhash, fee_calculator) = rpc_client.get_recent_blockhash()?;

    let ix = budget_instruction::apply_timestamp(&config.keypair.pubkey(), pubkey, to, dt);
    let mut tx = Transaction::new_signed_instructions(&[&config.keypair], vec![ix], blockhash);
    check_account_for_fee(
        rpc_client,
        &config.keypair.pubkey(),
        &fee_calculator,
        &tx.message,
    )?;
    let result = rpc_client.send_and_confirm_transaction(&mut tx, &[&config.keypair]);
    log_instruction_custom_error::<BudgetError>(result)
}

fn process_witness(
    rpc_client: &RpcClient,
    config: &CliConfig,
    to: &Pubkey,
    pubkey: &Pubkey,
) -> ProcessResult {
    let (blockhash, fee_calculator) = rpc_client.get_recent_blockhash()?;

    let ix = budget_instruction::apply_signature(&config.keypair.pubkey(), pubkey, to);
    let mut tx = Transaction::new_signed_instructions(&[&config.keypair], vec![ix], blockhash);
    check_account_for_fee(
        rpc_client,
        &config.keypair.pubkey(),
        &fee_calculator,
        &tx.message,
    )?;
    let result = rpc_client.send_and_confirm_transaction(&mut tx, &[&config.keypair]);
    log_instruction_custom_error::<BudgetError>(result)
}

pub fn process_command(config: &CliConfig) -> ProcessResult {
    if config.verbose {
        if let Some(keypair_path) = &config.keypair_path {
            println_name_value("Keypair:", keypair_path);
        }
        println_name_value("RPC Endpoint:", &config.json_rpc_url);
    }

    let mut _rpc_client;
    let rpc_client = if config.rpc_client.is_none() {
        _rpc_client = RpcClient::new(config.json_rpc_url.to_string());
        &_rpc_client
    } else {
        // Primarily for testing
        config.rpc_client.as_ref().unwrap()
    };

    match &config.command {
        // Cluster Query Commands
        // Get address of this client
        CliCommand::Address => Ok(format!("{}", config.keypair.pubkey())),

        // Return software version of solana-cli and cluster entrypoint node
        CliCommand::Catchup { node_pubkey } => process_catchup(&rpc_client, node_pubkey),
        CliCommand::ClusterVersion => process_cluster_version(&rpc_client),
        CliCommand::CreateAddressWithSeed {
            from_pubkey,
            seed,
            program_id,
        } => process_create_address_with_seed(config, from_pubkey.as_ref(), &seed, &program_id),
        CliCommand::Fees => process_fees(&rpc_client),
        CliCommand::GetBlockTime { slot } => process_get_block_time(&rpc_client, *slot),
        CliCommand::GetGenesisHash => process_get_genesis_hash(&rpc_client),
        CliCommand::GetEpochInfo { commitment_config } => {
            process_get_epoch_info(&rpc_client, commitment_config)
        }
        CliCommand::GetSlot { commitment_config } => {
            process_get_slot(&rpc_client, commitment_config)
        }
        CliCommand::GetTransactionCount { commitment_config } => {
            process_get_transaction_count(&rpc_client, commitment_config)
        }
        CliCommand::Ping {
            lamports,
            interval,
            count,
            timeout,
            commitment_config,
        } => process_ping(
            &rpc_client,
            config,
            *lamports,
            interval,
            count,
            timeout,
            commitment_config,
        ),
        CliCommand::ShowBlockProduction { epoch, slot_limit } => {
            process_show_block_production(&rpc_client, *epoch, *slot_limit)
        }
        CliCommand::ShowGossip => process_show_gossip(&rpc_client),
        CliCommand::ShowValidators { use_lamports_unit } => {
            process_show_validators(&rpc_client, *use_lamports_unit)
        }

        // Nonce Commands

        // Assign authority to nonce account
        CliCommand::AuthorizeNonceAccount {
            nonce_account,
            nonce_authority,
            new_authority,
        } => process_authorize_nonce_account(
            &rpc_client,
            config,
            nonce_account,
            nonce_authority,
            new_authority,
        ),
        // Create nonce account
        CliCommand::CreateNonceAccount {
            nonce_account,
            nonce_authority,
            lamports,
        } => process_create_nonce_account(
            &rpc_client,
            config,
            nonce_account,
            nonce_authority,
            *lamports,
        ),
        // Get the current nonce
        CliCommand::GetNonce(nonce_account_pubkey) => {
            process_get_nonce(&rpc_client, &nonce_account_pubkey)
        }
        // Get a new nonce
        CliCommand::NewNonce {
            nonce_account,
            nonce_authority,
        } => process_new_nonce(&rpc_client, config, nonce_account, nonce_authority),
        // Show the contents of a nonce account
        CliCommand::ShowNonceAccount {
            nonce_account_pubkey,
            use_lamports_unit,
        } => process_show_nonce_account(&rpc_client, &nonce_account_pubkey, *use_lamports_unit),
        // Withdraw lamports from a nonce account
        CliCommand::WithdrawFromNonceAccount {
            nonce_account,
            nonce_authority,
            destination_account_pubkey,
            lamports,
        } => process_withdraw_from_nonce_account(
            &rpc_client,
            config,
            &nonce_account,
            nonce_authority,
            &destination_account_pubkey,
            *lamports,
        ),

        // Program Deployment

        // Deploy a custom program to the chain
        CliCommand::Deploy(ref program_location) => {
            process_deploy(&rpc_client, config, program_location)
        }

        // Stake Commands

        // Create stake account
        CliCommand::CreateStakeAccount {
            stake_account,
            staker,
            withdrawer,
            lockup,
            lamports,
        } => process_create_stake_account(
            &rpc_client,
            config,
            stake_account,
            staker,
            withdrawer,
            lockup,
            *lamports,
        ),
        // Deactivate stake account
        CliCommand::DeactivateStake {
            stake_account_pubkey,
            sign_only,
            ref signers,
            blockhash,
        } => process_deactivate_stake_account(
            &rpc_client,
            config,
            &stake_account_pubkey,
            *sign_only,
            signers,
            *blockhash,
        ),
        CliCommand::DelegateStake {
            stake_account_pubkey,
            vote_account_pubkey,
            force,
            sign_only,
            ref signers,
            blockhash,
        } => process_delegate_stake(
            &rpc_client,
            config,
            &stake_account_pubkey,
            &vote_account_pubkey,
            *force,
            *sign_only,
            signers,
            *blockhash,
        ),
        CliCommand::RedeemVoteCredits(stake_account_pubkey, vote_account_pubkey) => {
            process_redeem_vote_credits(
                &rpc_client,
                config,
                &stake_account_pubkey,
                &vote_account_pubkey,
            )
        }
        CliCommand::ShowStakeAccount {
            pubkey: stake_account_pubkey,
            use_lamports_unit,
        } => process_show_stake_account(
            &rpc_client,
            config,
            &stake_account_pubkey,
            *use_lamports_unit,
        ),
        CliCommand::ShowStakeHistory { use_lamports_unit } => {
            process_show_stake_history(&rpc_client, config, *use_lamports_unit)
        }
        CliCommand::StakeAuthorize(
            stake_account_pubkey,
            new_authorized_pubkey,
            stake_authorize,
        ) => process_stake_authorize(
            &rpc_client,
            config,
            &stake_account_pubkey,
            &new_authorized_pubkey,
            *stake_authorize,
        ),

        CliCommand::WithdrawStake(stake_account_pubkey, destination_account_pubkey, lamports) => {
            process_withdraw_stake(
                &rpc_client,
                config,
                &stake_account_pubkey,
                &destination_account_pubkey,
                *lamports,
            )
        }

        // Storage Commands

        // Create storage account
        CliCommand::CreateStorageAccount {
            account_owner,
            storage_account,
            account_type,
        } => process_create_storage_account(
            &rpc_client,
            config,
            &account_owner,
            storage_account,
            *account_type,
        ),
        CliCommand::ClaimStorageReward {
            node_account_pubkey,
            storage_account_pubkey,
        } => process_claim_storage_reward(
            &rpc_client,
            config,
            node_account_pubkey,
            &storage_account_pubkey,
        ),
        CliCommand::ShowStorageAccount(storage_account_pubkey) => {
            process_show_storage_account(&rpc_client, config, &storage_account_pubkey)
        }

        // Validator Info Commands

        // Return all or single validator info
        CliCommand::GetValidatorInfo(info_pubkey) => {
            process_get_validator_info(&rpc_client, *info_pubkey)
        }
        // Publish validator info
        CliCommand::SetValidatorInfo {
            validator_info,
            force_keybase,
            info_pubkey,
        } => process_set_validator_info(
            &rpc_client,
            config,
            &validator_info,
            *force_keybase,
            *info_pubkey,
        ),

        // Vote Commands

        // Create vote account
        CliCommand::CreateVoteAccount {
            vote_account,
            node_pubkey,
            authorized_voter,
            authorized_withdrawer,
            commission,
        } => process_create_vote_account(
            &rpc_client,
            config,
            vote_account,
            &node_pubkey,
            authorized_voter,
            authorized_withdrawer,
            *commission,
        ),
        CliCommand::ShowVoteAccount {
            pubkey: vote_account_pubkey,
            use_lamports_unit,
        } => process_show_vote_account(
            &rpc_client,
            config,
            &vote_account_pubkey,
            *use_lamports_unit,
        ),
        CliCommand::VoteAuthorize {
            vote_account_pubkey,
            new_authorized_pubkey,
            vote_authorize,
        } => process_vote_authorize(
            &rpc_client,
            config,
            &vote_account_pubkey,
            &new_authorized_pubkey,
            *vote_authorize,
        ),
        CliCommand::VoteUpdateValidator {
            vote_account_pubkey,
            new_identity_pubkey,
            authorized_voter,
        } => process_vote_update_validator(
            &rpc_client,
            config,
            &vote_account_pubkey,
            &new_identity_pubkey,
            authorized_voter,
        ),
        CliCommand::Uptime {
            pubkey: vote_account_pubkey,
            aggregate,
            span,
        } => process_uptime(&rpc_client, config, &vote_account_pubkey, *aggregate, *span),

        // Wallet Commands

        // Request an airdrop from Solana Faucet;
        CliCommand::Airdrop {
            faucet_host,
            faucet_port,
            lamports,
            use_lamports_unit,
        } => {
            let faucet_addr = SocketAddr::new(
                faucet_host.unwrap_or_else(|| {
                    let faucet_host = url::Url::parse(&config.json_rpc_url)
                        .unwrap()
                        .host()
                        .unwrap()
                        .to_string();
                    solana_net_utils::parse_host(&faucet_host).unwrap_or_else(|err| {
                        panic!("Unable to resolve {}: {}", faucet_host, err);
                    })
                }),
                *faucet_port,
            );

            process_airdrop(
                &rpc_client,
                config,
                &faucet_addr,
                *lamports,
                *use_lamports_unit,
            )
        }
        // Check client balance
        CliCommand::Balance {
            pubkey,
            use_lamports_unit,
        } => process_balance(&rpc_client, config, &pubkey, *use_lamports_unit),
        // Cancel a contract by contract Pubkey
        CliCommand::Cancel(pubkey) => process_cancel(&rpc_client, config, &pubkey),
        // Confirm the last client transaction by signature
        CliCommand::Confirm(signature) => process_confirm(&rpc_client, signature),
        // If client has positive balance, pay lamports to another address
        CliCommand::Pay {
            lamports,
            to,
            timestamp,
            timestamp_pubkey,
            ref witnesses,
            cancelable,
            sign_only,
            ref signers,
            blockhash,
        } => process_pay(
            &rpc_client,
            config,
            *lamports,
            &to,
            *timestamp,
            *timestamp_pubkey,
            witnesses,
            *cancelable,
            *sign_only,
            signers,
            *blockhash,
        ),
        CliCommand::ShowAccount {
            pubkey,
            output_file,
            use_lamports_unit,
        } => process_show_account(
            &rpc_client,
            config,
            &pubkey,
            &output_file,
            *use_lamports_unit,
        ),
        // Apply time elapsed to contract
        CliCommand::TimeElapsed(to, pubkey, dt) => {
            process_time_elapsed(&rpc_client, config, &to, &pubkey, *dt)
        }
        // Apply witness signature to contract
        CliCommand::Witness(to, pubkey) => process_witness(&rpc_client, config, &to, &pubkey),
    }
}

// Quick and dirty Keypair that assumes the client will do retries but not update the
// blockhash. If the client updates the blockhash, the signature will be invalid.
struct FaucetKeypair {
    transaction: Transaction,
}

impl FaucetKeypair {
    fn new_keypair(
        faucet_addr: &SocketAddr,
        to_pubkey: &Pubkey,
        lamports: u64,
        blockhash: Hash,
    ) -> Result<Self, Box<dyn error::Error>> {
        let transaction = request_airdrop_transaction(faucet_addr, to_pubkey, lamports, blockhash)?;
        Ok(Self { transaction })
    }

    fn airdrop_transaction(&self) -> Transaction {
        self.transaction.clone()
    }
}

impl KeypairUtil for FaucetKeypair {
    fn new() -> Self {
        unimplemented!();
    }

    /// Return the public key of the keypair used to sign votes
    fn pubkey(&self) -> Pubkey {
        self.transaction.message().account_keys[0]
    }

    fn sign_message(&self, _msg: &[u8]) -> Signature {
        self.transaction.signatures[0]
    }
}

pub fn request_and_confirm_airdrop(
    rpc_client: &RpcClient,
    faucet_addr: &SocketAddr,
    to_pubkey: &Pubkey,
    lamports: u64,
) -> ProcessResult {
    let (blockhash, _fee_calculator) = rpc_client.get_recent_blockhash()?;
    let keypair = {
        let mut retries = 5;
        loop {
            let result = FaucetKeypair::new_keypair(faucet_addr, to_pubkey, lamports, blockhash);
            if result.is_ok() || retries == 0 {
                break result;
            }
            retries -= 1;
            sleep(Duration::from_secs(1));
        }
    }?;
    let mut tx = keypair.airdrop_transaction();
    let result = rpc_client.send_and_confirm_transaction(&mut tx, &[&keypair]);
    log_instruction_custom_error::<SystemError>(result)
}

pub fn log_instruction_custom_error<E>(result: Result<String, ClientError>) -> ProcessResult
where
    E: 'static + std::error::Error + DecodeError<E> + FromPrimitive,
{
    match result {
        Err(err) => {
            if let ClientError::TransactionError(TransactionError::InstructionError(
                _,
                InstructionError::CustomError(code),
            )) = err
            {
                if let Some(specific_error) = E::decode_custom_error_to_enum(code) {
                    error!("{}::{:?}", E::type_of(), specific_error);
                    return Err(specific_error.into());
                }
            }
            error!("{:?}", err);
            Err(err.into())
        }
        Ok(sig) => Ok(sig),
    }
}

// If clap arg `name` is_required, and specifies an amount of either lamports or SOL, the only way
// `amount_of()` can return None is if `name` is an f64 and `unit`== "lamports". This method
// catches that case and converts it to an Error.
pub(crate) fn required_lamports_from(
    matches: &ArgMatches<'_>,
    name: &str,
    unit: &str,
) -> Result<u64, CliError> {
    amount_of(matches, name, unit).ok_or_else(|| {
        CliError::BadParameter(format!(
            "Lamports cannot be fractional: {}",
            matches.value_of("amount").unwrap()
        ))
    })
}
