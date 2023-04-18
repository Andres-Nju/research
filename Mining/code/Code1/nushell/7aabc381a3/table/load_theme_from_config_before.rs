fn load_theme_from_config(config: &Config) -> TableTheme {
    match config.table_mode.as_str() {
        "basic" => nu_table::TableTheme::basic(),
        "compact" => nu_table::TableTheme::compact(),
        "compact_double" => nu_table::TableTheme::compact_double(),
        "light" => nu_table::TableTheme::light(),
        "with_love" => nu_table::TableTheme::with_love(),
        "rounded" => nu_table::TableTheme::rounded(),
        "reinforced" => nu_table::TableTheme::reinforced(),
        "heavy" => nu_table::TableTheme::heavy(),
        "none" => nu_table::TableTheme::none(),
        _ => nu_table::TableTheme::rounded(),
    }
}
