pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .arg(
            Arg::new(OPT_DATE)
                .short('d')
                .long(OPT_DATE)
                .value_name("STRING")
                .help("display time described by STRING, not 'now'"),
        )
        .arg(
            Arg::new(OPT_FILE)
                .short('f')
                .long(OPT_FILE)
                .value_name("DATEFILE")
                .value_hint(clap::ValueHint::FilePath)
                .help("like --date; once for each line of DATEFILE"),
        )
        .arg(
            Arg::new(OPT_ISO_8601)
                .short('I')
                .long(OPT_ISO_8601)
                .value_name("FMT")
                .value_parser([DATE, HOUR, HOURS, MINUTE, MINUTES, SECOND, SECONDS, NS])
                .num_args(0..=1)
                .default_missing_value(OPT_DATE)
                .help(ISO_8601_HELP_STRING),
        )
        .arg(
            Arg::new(OPT_RFC_EMAIL)
                .short('R')
                .long(OPT_RFC_EMAIL)
                .help(RFC_5322_HELP_STRING)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_RFC_3339)
                .long(OPT_RFC_3339)
                .value_name("FMT")
                .help(RFC_3339_HELP_STRING),
        )
        .arg(
            Arg::new(OPT_DEBUG)
                .long(OPT_DEBUG)
                .help("annotate the parsed date, and warn about questionable usage to stderr")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(OPT_REFERENCE)
                .short('r')
                .long(OPT_REFERENCE)
                .value_name("FILE")
                .value_hint(clap::ValueHint::AnyPath)
                .help("display the last modification time of FILE"),
        )
        .arg(
            Arg::new(OPT_SET)
                .short('s')
                .long(OPT_SET)
                .value_name("STRING")
                .help(OPT_SET_HELP_STRING),
        )
        .arg(
            Arg::new(OPT_UNIVERSAL)
                .short('u')
                .long(OPT_UNIVERSAL)
                .alias(OPT_UNIVERSAL_2)
                .help("print or set Coordinated Universal Time (UTC)")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new(OPT_FORMAT))
}
