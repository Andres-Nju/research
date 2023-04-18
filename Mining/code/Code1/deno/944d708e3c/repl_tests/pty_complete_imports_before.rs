fn pty_complete_imports() {
  util::with_pty(&["repl"], |mut console| {
    // single quotes
    console.write_line("import './001_hel\t'");
    // double quotes
    console.write_line("import { output } from \"./045_out\t\"");
    console.write_line("output('testing output');");
    console.write_line("close();");

    let output = console.read_all_output();
    assert_contains!(output, "Hello World");
    assert_contains!(
      output,
      // on windows, could contain either (it's flaky)
      "\ntesting output",
      "testing output\u{1b}",
    );
  });

  // ensure when the directory changes that the suggestions come from the cwd
  util::with_pty(&["repl"], |mut console| {
    console.write_line("Deno.chdir('./subdir');");
    console.write_line("import '../001_hel\t'");
    console.write_line("close();");

    let output = console.read_all_output();
    assert_contains!(output, "Hello World");
  });

  // does not panic when tabbing when empty
  util::with_pty(&["repl"], |mut console| {
    console.write_line("import '\t");
  });
}
