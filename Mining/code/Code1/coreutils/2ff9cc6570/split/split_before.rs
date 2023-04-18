fn split(settings: &Settings) -> i32 {
    let mut reader = BufReader::new(if settings.input == "-" {
        Box::new(stdin()) as Box<dyn Read>
    } else {
        let r = File::open(Path::new(&settings.input)).unwrap_or_else(|_| {
            crash!(
                1,
                "cannot open '{}' for reading: No such file or directory",
                settings.input
            )
        });
        Box::new(r) as Box<dyn Read>
    });

    let mut splitter: Box<dyn Splitter> = match settings.strategy.as_str() {
        s if s == OPT_LINES => Box::new(LineSplitter::new(settings)),
        s if (s == OPT_BYTES || s == OPT_LINE_BYTES) => Box::new(ByteSplitter::new(settings)),
        a => crash!(1, "strategy {} not supported", a),
    };

    let mut fileno = 0;
    loop {
        // Get a new part file set up, and construct `writer` for it.
        let mut filename = settings.prefix.clone();
        filename.push_str(
            if settings.numeric_suffix {
                num_prefix(fileno, settings.suffix_length)
            } else {
                str_prefix(fileno, settings.suffix_length)
            }
            .as_ref(),
        );
        filename.push_str(settings.additional_suffix.as_ref());
        let mut writer = platform::instantiate_current_writer(&settings.filter, filename.as_str());

        let bytes_consumed = splitter.consume(&mut reader, &mut writer);
        writer
            .flush()
            .unwrap_or_else(|e| crash!(1, "error flushing to output file: {}", e));

        // If we didn't write anything we should clean up the empty file, and
        // break from the loop.
        if bytes_consumed == 0 {
            // The output file is only ever created if filter's aren't used.
            // Complicated, I know...
            if settings.filter.is_none() {
                remove_file(filename)
                    .unwrap_or_else(|e| crash!(1, "error removing empty file: {}", e));
            }
            break;
        }

        fileno += 1;
    }
    0
}
