fn folder_with_init_py() -> io::Result<()> {
    let dir = tempfile::tempdir()?;
    File::create(dir.path().join("__init__.py"))?.sync_all()?;

    let output = common::render_module("python")
        .arg("--path")
        .arg(dir.path())
        .output()?;
    let actual = String::from_utf8(output.stdout).unwrap();

    let expected = format!("via {} ", Color::Yellow.bold().paint("🐍 v3.7.5"));
    assert_eq!(expected, actual);
    Ok(())
}
