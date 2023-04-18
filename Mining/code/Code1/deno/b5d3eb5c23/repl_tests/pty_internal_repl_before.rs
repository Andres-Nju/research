fn pty_internal_repl() {
  util::with_pty(&["repl"], |mut console| {
    console.write_line("globalThis");
    console.write_line_raw("1 + 256");
    let output = console.read_until("257");
    assert_contains!(output, "clear:");
    assert_not_contains!(output, "__DENO_");

    console.write_line_raw("__\t\t");
    console.expect("> __");
    let output = console.read_until("> __");
    assert_contains!(output, "__defineGetter__");
    // should not contain the internal repl variable
    // in the `globalThis` or completions output
    assert_not_contains!(output, "__DENO_");
  });
}
