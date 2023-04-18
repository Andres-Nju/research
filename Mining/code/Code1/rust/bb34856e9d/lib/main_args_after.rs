pub fn main_args(args: &[String]) -> isize {
    let all_groups: Vec<getopts::OptGroup> = opts()
                                             .into_iter()
                                             .map(|x| x.opt_group)
                                             .collect();
    let matches = match getopts::getopts(&args[1..], &all_groups) {
        Ok(m) => m,
        Err(err) => {
            print_error(err);
            return 1;
        }
    };
    // Check for unstable options.
    nightly_options::check_nightly_options(&matches, &opts());

    if matches.opt_present("h") || matches.opt_present("help") {
        usage("rustdoc");
        return 0;
    } else if matches.opt_present("version") {
        rustc_driver::version("rustdoc", &matches);
        return 0;
    }

    if matches.opt_strs("passes") == ["list"] {
        println!("Available passes for running rustdoc:");
        for &(name, _, description) in passes::PASSES {
            println!("{:>20} - {}", name, description);
        }
        println!("\nDefault passes for rustdoc:");
        for &name in passes::DEFAULT_PASSES {
            println!("{:>20}", name);
        }
        return 0;
    }

    if matches.free.is_empty() {
        print_error("missing file operand");
        return 1;
    }
    if matches.free.len() > 1 {
        print_error("too many file operands");
        return 1;
    }
    let input = &matches.free[0];

    let mut libs = SearchPaths::new();
    for s in &matches.opt_strs("L") {
        libs.add_path(s, ErrorOutputType::default());
    }
    let externs = match parse_externs(&matches) {
        Ok(ex) => ex,
        Err(err) => {
            print_error(err);
            return 1;
        }
    };

    let test_args = matches.opt_strs("test-args");
    let test_args: Vec<String> = test_args.iter()
                                          .flat_map(|s| s.split_whitespace())
                                          .map(|s| s.to_string())
                                          .collect();

    let should_test = matches.opt_present("test");
    let markdown_input = input.ends_with(".md") || input.ends_with(".markdown");

    let output = matches.opt_str("o").map(|s| PathBuf::from(&s));
    let css_file_extension = matches.opt_str("e").map(|s| PathBuf::from(&s));
    let cfgs = matches.opt_strs("cfg");

    if let Some(ref p) = css_file_extension {
        if !p.is_file() {
            writeln!(
                &mut io::stderr(),
                "rustdoc: option --extend-css argument must be a file."
            ).unwrap();
            return 1;
        }
    }

    let external_html = match ExternalHtml::load(
            &matches.opt_strs("html-in-header"),
            &matches.opt_strs("html-before-content"),
            &matches.opt_strs("html-after-content")) {
        Some(eh) => eh,
        None => return 3,
    };
    let crate_name = matches.opt_str("crate-name");
    let playground_url = matches.opt_str("playground-url");
    let maybe_sysroot = matches.opt_str("sysroot").map(PathBuf::from);

    match (should_test, markdown_input) {
        (true, true) => {
            return markdown::test(input, cfgs, libs, externs, test_args, maybe_sysroot)
        }
        (true, false) => {
            return test::run(input, cfgs, libs, externs, test_args, crate_name, maybe_sysroot)
        }
        (false, true) => return markdown::render(input,
                                                 output.unwrap_or(PathBuf::from("doc")),
                                                 &matches, &external_html,
                                                 !matches.opt_present("markdown-no-toc")),
        (false, false) => {}
    }

    let output_format = matches.opt_str("w");
    let res = acquire_input(input, externs, &matches, move |out| {
        let Output { krate, passes, renderinfo } = out;
        info!("going to format");
        match output_format.as_ref().map(|s| &**s) {
            Some("html") | None => {
                html::render::run(krate, &external_html, playground_url,
                                  output.unwrap_or(PathBuf::from("doc")),
                                  passes.into_iter().collect(),
                                  css_file_extension,
                                  renderinfo)
                    .expect("failed to generate documentation");
                0
            }
            Some(s) => {
                print_error(format!("unknown output format: {}", s));
                1
            }
        }
    });
    res.unwrap_or_else(|s| {
        print_error(format!("input error: {}", s));
        1
    })
}
