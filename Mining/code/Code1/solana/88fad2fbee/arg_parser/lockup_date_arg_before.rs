fn lockup_date_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("lockup_date")
        .long("lockup-date")
        .value_name("RFC3339 DATETIME")
        .validator(is_rfc3339_datetime)
        .takes_value(true)
        .help("The date and time at which each account will be available for withdrawl")
}
