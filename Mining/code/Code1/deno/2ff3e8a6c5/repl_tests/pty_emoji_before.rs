fn pty_emoji() {
  // windows was having issues displaying this
  util::with_pty(&["repl"], |mut console| {
    console.write_line("console.log('🦕');");
    console.write_line("close();");

    let output = console.read_all_output();
    // one for input, one for output
    let emoji_count = output.chars().filter(|c| *c == '🦕').count();
    assert_eq!(emoji_count, 2);
  });
}
