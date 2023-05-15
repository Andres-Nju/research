fn read_config(path: &PathBuf) -> Result<Config> {
    let mut contents = fs::read_to_string(path)?;

    // Remove UTF-8 BOM
    if contents.starts_with('\u{FEFF}') {
        contents = contents.split_off(3);
    }

    parse_config(&contents)
}