fn parse_settings(matches: &ArgMatches<'_>) -> Result<bool, Box<dyn error::Error>> {
    let parse_args = match matches.subcommand() {
        ("get", Some(subcommand_matches)) => {
            if let Some(config_file) = matches.value_of("config_file") {
                let config = Config::load(config_file).unwrap_or_default();
                if let Some(field) = subcommand_matches.value_of("specific_setting") {
                    let (value, default_value) = match field {
                        "url" => (config.url, CliConfig::default_json_rpc_url()),
                        "keypair" => (config.keypair_path, CliConfig::default_keypair_path()),
                        _ => unreachable!(),
                    };
                    println_name_value_or(&format!("* {}:", field), &value, &default_value);
                } else {
                    println_name_value("Wallet Config:", config_file);
                    println_name_value_or(
                        "* url:",
                        &config.url,
                        &CliConfig::default_json_rpc_url(),
                    );
                    println_name_value_or(
                        "* keypair:",
                        &config.keypair_path,
                        &CliConfig::default_keypair_path(),
                    );
                }
            } else {
                println!(
                    "{} Either provide the `--config` arg or ensure home directory exists to use the default config location",
                    style("No config file found.").bold()
                );
            }
            false
        }
        ("set", Some(subcommand_matches)) => {
            if let Some(config_file) = matches.value_of("config_file") {
                let mut config = Config::load(config_file).unwrap_or_default();
                if let Some(url) = subcommand_matches.value_of("json_rpc_url") {
                    config.url = url.to_string();
                }
                if let Some(keypair) = subcommand_matches.value_of("keypair") {
                    config.keypair_path = keypair.to_string();
                }
                config.save(config_file)?;
                println_name_value("Wallet Config Updated:", config_file);
                println_name_value("* url:", &config.url);
                println_name_value("* keypair:", &config.keypair_path);
            } else {
                println!(
                    "{} Either provide the `--config` arg or ensure home directory exists to use the default config location",
                    style("No config file found.").bold()
                );
            }
            false
        }
        _ => true,
    };
    Ok(parse_args)
}

pub fn parse_args(matches: &ArgMatches<'_>) -> Result<CliConfig, Box<dyn error::Error>> {
    let config = if let Some(config_file) = matches.value_of("config_file") {
        Config::load(config_file).unwrap_or_default()
    } else {
        Config::default()
    };
    let json_rpc_url = if let Some(url) = matches.value_of("json_rpc_url") {
        url.to_string()
    } else if config.url != "" {
        config.url
    } else {
        let default = CliConfig::default();
        default.json_rpc_url
    };

    let CliCommandInfo {
        command,
        require_keypair,
    } = parse_command(&matches)?;

    let (keypair, keypair_path) = if require_keypair {
        let KeypairWithSource { keypair, source } = keypair_input(&matches, "keypair")?;
        match source {
            keypair::Source::File => (
                keypair,
                Some(matches.value_of("keypair").unwrap().to_string()),
            ),
            keypair::Source::SeedPhrase => (keypair, None),
            keypair::Source::Generated => {
                let keypair_path = if config.keypair_path != "" {
                    config.keypair_path
                } else {
                    let default_keypair_path = CliConfig::default_keypair_path();
                    if !std::path::Path::new(&default_keypair_path).exists() {
                        return Err(CliError::KeypairFileNotFound(
                            "Generate a new keypair with `solana-keygen new`".to_string(),
                        )
                        .into());
                    }
                    default_keypair_path
                };

                let keypair = read_keypair_file(&keypair_path).or_else(|err| {
                    Err(CliError::BadParameter(format!(
                        "{}: Unable to open keypair file: {}",
                        err, keypair_path
                    )))
                })?;

                (keypair, Some(keypair_path))
            }
        }
    } else {
        let default = CliConfig::default();
        (default.keypair, None)
    };

    Ok(CliConfig {
        command,
        json_rpc_url,
        keypair,
        keypair_path,
        rpc_client: None,
        verbose: matches.is_present("verbose"),
    })
}
