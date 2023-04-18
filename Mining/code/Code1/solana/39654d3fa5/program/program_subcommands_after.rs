    fn program_subcommands(self) -> Self {
        self.subcommand(
            SubCommand::with_name("program")
                .about("Program management")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("deploy")
                        .about("Deploy a program")
                        .arg(
                            Arg::with_name("program_location")
                                .index(1)
                                .value_name("PROGRAM_FILEPATH")
                                .takes_value(true)
                                .help("/path/to/program.so"),
                        )
                        .arg(
                            Arg::with_name("buffer")
                                .long("buffer")
                                .value_name("BUFFER_SIGNER")
                                .takes_value(true)
                                .validator(is_valid_signer)
                                .help("Intermediate buffer account to write data to, which can be used to resume a failed deploy \
                                      [default: random address]")
                        )
                        .arg(
                            Arg::with_name("upgrade_authority")
                                .long("upgrade-authority")
                                .value_name("UPGRADE_AUTHORITY_SIGNER")
                                .takes_value(true)
                                .validator(is_valid_signer)
                                .help("Upgrade authority [default: the default configured keypair]")
                        )
                        .arg(
                            pubkey!(Arg::with_name("program_id")
                                .long("program-id")
                                .value_name("PROGRAM_ID"),
                                "Executable program's address, must be a keypair for initial deploys, can be a pubkey for upgrades \
                                [default: address of keypair at /path/to/program-keypair.json if present, otherwise a random address]"),
                        )
                        .arg(
                            Arg::with_name("final")
                                .long("final")
                                .help("The program will not be upgradeable")
                        )
                        .arg(
                            Arg::with_name("max_len")
                                .long("max-len")
                                .value_name("max_len")
                                .takes_value(true)
                                .required(false)
                                .help("Maximum length of the upgradeable program \
                                      [default: twice the length of the original deployed program]")
                        )
                        .arg(
                            Arg::with_name("allow_excessive_balance")
                                .long("allow-excessive-deploy-account-balance")
                                .takes_value(false)
                                .help("Use the designated program id even if the account already holds a large balance of SOL")
                        ),
                )
                .subcommand(
                    SubCommand::with_name("write-buffer")
                        .about("Writes a program into a buffer account")
                        .arg(
                            Arg::with_name("program_location")
                                .index(1)
                                .value_name("PROGRAM_FILEPATH")
                                .takes_value(true)
                                .required(true)
                                .help("/path/to/program.so"),
                        )
                        .arg(
                            Arg::with_name("buffer")
                                .long("buffer")
                                .value_name("BUFFER_SIGNER")
                                .takes_value(true)
                                .validator(is_valid_signer)
                                .help("Buffer account to write data into [default: random address]")
                        )
                        .arg(
                            Arg::with_name("buffer_authority")
                                .long("buffer-authority")
                                .value_name("BUFFER_AUTHORITY_SIGNER")
                                .takes_value(true)
                                .validator(is_valid_signer)
                                .help("Buffer authority [default: the default configured keypair]")
                        )
                        .arg(
                            Arg::with_name("max_len")
                                .long("max-len")
                                .value_name("max_len")
                                .takes_value(true)
                                .required(false)
                                .help("Maximum length of the upgradeable program \
                                      [default: twice the length of the original deployed program]")
                        ),
                )
                .subcommand(
                    SubCommand::with_name("set-buffer-authority")
                        .about("Set a new buffer authority")
                        .arg(
                            Arg::with_name("buffer")
                                .index(1)
                                .value_name("BUFFER_PUBKEY")
                                .takes_value(true)
                                .required(true)
                                .help("Public key of the buffer")
                        )
                        .arg(
                            Arg::with_name("buffer_authority")
                                .long("buffer-authority")
                                .value_name("BUFFER_AUTHORITY_SIGNER")
                                .takes_value(true)
                                .validator(is_valid_signer)
                                .help("Buffer authority [default: the default configured keypair]")
                        )
                        .arg(
                            pubkey!(Arg::with_name("new_buffer_authority")
                                .long("new-buffer-authority")
                                .value_name("NEW_BUFFER_AUTHORITY")
                                .required(true),
                                "Address of the new buffer authority"),
                        )
                )
                .subcommand(
                    SubCommand::with_name("set-upgrade-authority")
                        .about("Set a new program authority")
                        .arg(
                            Arg::with_name("program_id")
                                .index(1)
                                .value_name("PROGRAM_ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Address of the program to upgrade")
                        )
                        .arg(
                            Arg::with_name("upgrade_authority")
                                .long("upgrade-authority")
                                .value_name("UPGRADE_AUTHORITY_SIGNER")
                                .takes_value(true)
                                .validator(is_valid_signer)
                                .help("Upgrade authority [default: the default configured keypair]")
                        )
                        .arg(
                            pubkey!(Arg::with_name("new_upgrade_authority")
                                .long("new-upgrade-authority")
                                .required_unless("final")
                                .value_name("NEW_UPGRADE_AUTHORITY"),
                                "Address of the new upgrade authority"),
                        )
                        .arg(
                            Arg::with_name("final")
                                .long("final")
                                .conflicts_with("new_upgrade_authority")
                                .help("The program will not be upgradeable")
                        )
                )
                .subcommand(
                    SubCommand::with_name("show")
                        .about("Display information about a buffer or program")
                        .arg(
                            Arg::with_name("account")
                                .index(1)
                                .value_name("ACCOUNT_ADDRESS")
                                .takes_value(true)
                                .help("Address of the buffer or program to show")
                        )
                        .arg(
                            Arg::with_name("buffers")
                                .long("buffers")
                                .conflicts_with("account")
                                .required_unless("account")
                                .help("Show every buffer account that matches the authority")
                        )
                        .arg(
                            Arg::with_name("all")
                                .long("all")
                                .conflicts_with("account")
                                .help("Show accounts for all authorities")
                        )
                        .arg(
                            pubkey!(Arg::with_name("buffer_authority")
                                .long("buffer-authority")
                                .value_name("AUTHORITY")
                                .conflicts_with("all"),
                                "Authority [default: the default configured keypair]"),
                        )
                        .arg(
                            Arg::with_name("lamports")
                                .long("lamports")
                                .takes_value(false)
                                .help("Display balance in lamports instead of SOL"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("dump")
                        .about("Write the program data to a file")
                        .arg(
                            Arg::with_name("account")
                                .index(1)
                                .value_name("ACCOUNT_ADDRESS")
                                .takes_value(true)
                                .required(true)
                                .help("Address of the buffer or program")
                        )
                        .arg(
                            Arg::with_name("output_location")
                                .index(2)
                                .value_name("OUTPUT_FILEPATH")
                                .takes_value(true)
                                .required(true)
                                .help("/path/to/program.so"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("close")
                        .about("Close an acount and withdraw all lamports")
                        .arg(
                            Arg::with_name("account")
                                .index(1)
                                .value_name("BUFFER_ACCOUNT_ADDRESS")
                                .takes_value(true)
                                .help("Address of the buffer account to close"),
                        )
                        .arg(
                            Arg::with_name("buffers")
                                .long("buffers")
                                .conflicts_with("account")
                                .required_unless("account")
                                .help("Close every buffer accounts that match the authority")
                        )
                        .arg(
                            Arg::with_name("buffer_authority")
                                .long("buffer-authority")
                                .value_name("AUTHORITY_SIGNER")
                                .takes_value(true)
                                .validator(is_valid_signer)
                                .help("Authority [default: the default configured keypair]")
                        )
                        .arg(
                            pubkey!(Arg::with_name("recipient_account")
                                .long("recipient")
                                .value_name("RECIPIENT_ADDRESS"),
                                "Address of the account to deposit the closed account's lamports [default: the default configured keypair]"),
                        )
                        .arg(
                            Arg::with_name("lamports")
                                .long("lamports")
                                .takes_value(false)
                                .help("Display balance in lamports instead of SOL"),
                        ),
                )
        )
    }
