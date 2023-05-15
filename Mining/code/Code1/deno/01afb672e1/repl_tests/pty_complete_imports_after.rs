  fn pty_complete_imports() {
    util::with_pty(&["repl", "-A"], |mut console| {
      // single quotes
      console.write_line("import './run/001_hel\t'");
      // double quotes
      console.write_line("import { output } from \"./run/045_out\t\"");
      console.write_line("output('testing output');");
      console.write_line("close();");

      let output = console.read_all_output();
      assert_contains!(output, "Hello World");
      assert_contains!(
        output,
        // on windows, could any (it's flaky)
        "\ntesting output",
        "testing output\u{1b}",
        "\r\n\u{1b}[?25htesting output",
      );
    });

    // ensure when the directory changes that the suggestions come from the cwd
    util::with_pty(&["repl", "-A"], |mut console| {
      console.write_line("Deno.chdir('./subdir');");
      console.write_line("import '../run/001_hel\t'");
      console.write_line("close();");

      let output = console.read_all_output();
      assert_contains!(output, "Hello World");
    });
  }