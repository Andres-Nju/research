fn main() -> Result<()> {
    Logger::with_env().start()?;

    let subcommand = match std::env::args_os().nth(1) {
        None => {
            eprintln!("{}", help::GLOBAL_HELP);
            return Ok(());
        }
        Some(s) => s,
    };
    let mut matches = Arguments::from_vec(std::env::args_os().skip(2).collect());

    match &*subcommand.to_string_lossy() {
        "parse" => {
            if matches.contains(["-h", "--help"]) {
                eprintln!("{}", help::PARSE_HELP);
                return Ok(());
            }
            let no_dump = matches.contains("--no-dump");
            matches.finish().or_else(handle_extra_flags)?;

            let _p = profile("parsing");
            let file = file()?;
            if !no_dump {
                println!("{:#?}", file.syntax());
            }
            std::mem::forget(file);
        }
        "symbols" => {
            if matches.contains(["-h", "--help"]) {
                eprintln!("{}", help::SYMBOLS_HELP);
                return Ok(());
            }
            matches.finish().or_else(handle_extra_flags)?;
            let file = file()?;
            for s in file_structure(&file) {
                println!("{:?}", s);
            }
        }
        "highlight" => {
            if matches.contains(["-h", "--help"]) {
                eprintln!("{}", help::HIGHLIGHT_HELP);
                return Ok(());
            }
            let rainbow_opt = matches.contains(["-r", "--rainbow"]);
            matches.finish().or_else(handle_extra_flags)?;
            let (analysis, file_id) = Analysis::from_single_file(read_stdin()?);
            let html = analysis.highlight_as_html(file_id, rainbow_opt).unwrap();
            println!("{}", html);
        }
        "analysis-stats" => {
            if matches.contains(["-h", "--help"]) {
                eprintln!("{}", help::ANALYSIS_STATS_HELP);
                return Ok(());
            }
            let verbosity = match (
                matches.contains(["-v", "--verbose"]),
                matches.contains(["-q", "--quiet"]),
            ) {
                (false, false) => Verbosity::Normal,
                (false, true) => Verbosity::Quiet,
                (true, false) => Verbosity::Verbose,
                (true, true) => Err("Invalid flags: -q conflicts with -v")?,
            };
            let memory_usage = matches.contains("--memory-usage");
            let only = matches.value_from_str(["-o", "--only"])?.map(|v: String| v.to_owned());
            let path = {
                let mut trailing = matches.free()?;
                if trailing.len() != 1 {
                    eprintln!("{}", help::ANALYSIS_STATS_HELP);
                    Err("Invalid flags")?;
                }
                trailing.pop().unwrap()
            };
            analysis_stats::run(
                verbosity,
                memory_usage,
                path.as_ref(),
                only.as_ref().map(String::as_ref),
            )?;
        }
        "analysis-bench" => {
            if matches.contains(["-h", "--help"]) {
                eprintln!("{}", help::ANALYSIS_BENCH_HELP);
                return Ok(());
            }
            let verbose = matches.contains(["-v", "--verbose"]);
            let path: String = matches.value_from_str("--path")?.unwrap_or_default();
            let highlight_path = matches.value_from_str("--highlight")?;
            let complete_path = matches.value_from_str("--complete")?;
            if highlight_path.is_some() && complete_path.is_some() {
                panic!("either --highlight or --complete must be set, not both")
            }
            let op = if let Some(path) = highlight_path {
                let path: String = path;
                analysis_bench::Op::Highlight { path: path.into() }
            } else if let Some(path_line_col) = complete_path {
                let path_line_col: String = path_line_col;
                let (path_line, column) = rsplit_at_char(path_line_col.as_str(), ':')?;
                let (path, line) = rsplit_at_char(path_line, ':')?;
                analysis_bench::Op::Complete {
                    path: path.into(),
                    line: line.parse()?,
                    column: column.parse()?,
                }
            } else {
                panic!("either --highlight or --complete must be set")
            };
            matches.finish().or_else(handle_extra_flags)?;
            analysis_bench::run(verbose, path.as_ref(), op)?;
        }
        _ => eprintln!("{}", help::GLOBAL_HELP),
    }
    Ok(())
}
