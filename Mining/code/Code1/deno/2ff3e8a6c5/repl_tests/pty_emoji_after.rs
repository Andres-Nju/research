fn pty_emoji() {
  // windows was having issues displaying this
  util::with_pty(&["repl"], |mut console| {
    console.write_line(r#"console.log('\u{1F995}');"#);
    console.write_line("close();");

    let output = console.read_all_output();
    // only one for the output (since input is escaped)
    let emoji_count = output.chars().filter(|c| *c == '🦕').count();
    assert_eq!(emoji_count, 1);
  });
}
