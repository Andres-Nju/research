pub fn list_themes(cfg: &Config) -> Result<()> {
    let assets = assets_from_cache_or_binary()?;
    let mut config = cfg.clone();
    let mut style = HashSet::new();
    style.insert(StyleComponent::Plain);
    config.language = Some("Rust");
    config.style_components = StyleComponents(style);

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if config.colored_output {
        for theme in assets.themes() {
            writeln!(
                stdout,
                "Theme: {}\n",
                Style::new().bold().paint(theme.to_string())
            )?;
            config.theme = theme.to_string();
            Controller::new(&config, &assets)
                .run(vec![theme_preview_file()])
                .ok();
            writeln!(stdout)?;
        }
        writeln!(
            stdout,
            "Further themes can be installed to '{}', \
            and are added to the cache with `bat cache --build`. \
            For more information, see:\n\n  \
            https://github.com/sharkdp/bat#adding-new-themes",
            PROJECT_DIRS.config_dir().join("themes").to_string_lossy()
        )?;
    } else {
        for theme in assets.themes() {
            writeln!(stdout, "{}", theme)?;
        }
    }

    Ok(())
}
