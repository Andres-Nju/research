fn disabled_scan_for_pyfiles_and_folder_with_setup_py() -> io::Result<()> {
    let dir = tempfile::tempdir()?;
    File::create(dir.path().join("setup.py"))?.sync_all()?;

    let output = common::render_module("python")
        .use_config(toml::toml! {
            [python]
            scan_for_pyfiles = false
        })
        .arg("--path")
        .arg(dir.path())
        .output()?;
    let actual = String::from_utf8(output.stdout).unwrap();

    let expected = format!("via {} ", Color::Yellow.bold().paint("🐍 v3.7.6"));
    assert_eq!(expected, actual);
    Ok(())
}
