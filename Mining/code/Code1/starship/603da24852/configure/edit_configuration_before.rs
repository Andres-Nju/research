pub fn edit_configuration() {
    let config_path = get_config_path();
    let editor_cmd = shell_words::split(&get_editor()).expect("Unmatched quotes found in $EDITOR.");

    let command = Command::new(&editor_cmd[0])
        .args(&editor_cmd[1..])
        .arg(config_path)
        .status();

    match command {
        Ok(_) => (),
        Err(error) => match error.kind() {
            ErrorKind::NotFound => {
                eprintln!(
                    "Error: editor {:?} was not found. Did you set your $EDITOR or $VISUAL \
                    environment variables correctly?",
                    editor_cmd
                );
                std::process::exit(1)
            }
            other_error => panic!("failed to open file: {:?}", other_error),
        },
    };
}
