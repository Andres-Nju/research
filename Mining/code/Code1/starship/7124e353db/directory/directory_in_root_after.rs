fn directory_in_root() -> io::Result<()> {
    let output = common::render_module("directory")
        .arg("--path=/etc")
        .output()?;
    let actual = String::from_utf8(output.stdout).unwrap();

    let expected = format!("in {} ", Color::Cyan.bold().paint("/etc"));
    assert_eq!(expected, actual);
    Ok(())
}
