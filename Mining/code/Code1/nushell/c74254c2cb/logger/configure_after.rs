pub fn configure(
    level: &str,
    target: &str,
    builder: &mut ConfigBuilder,
) -> (LevelFilter, LogTarget) {
    let level = match Level::from_str(level) {
        Ok(level) => level,
        Err(_) => Level::Warn,
    };

    // Add allowed module filter
    builder.add_filter_allow_str("nu");

    // Set level padding
    builder.set_level_padding(LevelPadding::Right);

    // Custom time format
    builder.set_time_format_custom(format_description!(
        "[year]-[month]-[day] [hour repr:12]:[minute]:[second].[subsecond digits:3] [period]"
    ));

    // Show module path
    builder.set_target_level(LevelFilter::Error);

    // Don't show thread id
    builder.set_thread_level(LevelFilter::Off);

    let log_target = LogTarget::from(target);

    // Only TermLogger supports color output
    if matches!(
        log_target,
        LogTarget::Stdout | LogTarget::Stderr | LogTarget::Mixed
    ) {
        Level::iter().for_each(|level| set_colored_level(builder, level));
    }

    (level.to_level_filter(), log_target)
}
