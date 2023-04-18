fn lockup_epoch_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("lockup_epoch")
        .long("lockup-epoch")
        .takes_value(true)
        .value_name("NUMBER")
        .help("The epoch height at which each account will be available for withdrawl")
}
