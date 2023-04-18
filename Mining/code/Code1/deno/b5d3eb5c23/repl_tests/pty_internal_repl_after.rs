fn pty_internal_repl() {
  util::with_pty(&["repl"], |mut console| {
    console.write_line("'Length: ' + Object.keys(globalThis).filter(k => k.startsWith('__DENO_')).length;");
    console.expect("Length: 0");

    console.write_line_raw("__\t\t");
    console.expect("> __");
    let output = console.read_until("> __");
    assert_contains!(output, "__defineGetter__");
    // should not contain the internal repl variable
    // in the `globalThis` or completions output
    assert_not_contains!(output, "__DENO_");
  });
}
