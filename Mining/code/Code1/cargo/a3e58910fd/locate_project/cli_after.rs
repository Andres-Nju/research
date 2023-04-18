pub fn cli() -> App {
    subcommand("locate-project")
        .about("Print a JSON representation of a Cargo.toml file's location")
        .arg_manifest_path()
}
