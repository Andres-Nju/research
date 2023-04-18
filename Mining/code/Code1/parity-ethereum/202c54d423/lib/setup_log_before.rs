pub fn setup_log(config: &Config) -> Result<Arc<RotatingLogger>, String> {
	use rlog::*;

	let mut levels = String::new();
	let mut builder = LogBuilder::new();
	// Disable info logging by default for some modules:
	builder.filter(Some("ws"), LogLevelFilter::Warn);
	builder.filter(Some("reqwest"), LogLevelFilter::Warn);
	builder.filter(Some("hyper"), LogLevelFilter::Warn);
	builder.filter(Some("rustls"), LogLevelFilter::Warn);
	// Enable info for others.
	builder.filter(None, LogLevelFilter::Info);

	if let Ok(lvl) = env::var("RUST_LOG") {
		levels.push_str(&lvl);
		levels.push_str(",");
		builder.parse(&lvl);
	}

	if let Some(ref s) = config.mode {
		levels.push_str(s);
		builder.parse(s);
	}

	let isatty = atty::is(atty::Stream::Stderr);
	let enable_color = config.color && isatty;
	let logs = Arc::new(RotatingLogger::new(levels));
	let logger = logs.clone();
	let mut open_options = fs::OpenOptions::new();

	let maybe_file = match config.file.as_ref() {
		Some(f) => Some(open_options
			.append(true).create(true).open(f)
			.map_err(|_| format!("Cannot write to log file given: {}", f))?),
		None => None,
	};

	let format = move |record: &LogRecord| {
		let timestamp = time::strftime("%Y-%m-%d %H:%M:%S %Z", &time::now()).unwrap();

		let with_color = if max_log_level() <= LogLevelFilter::Info {
			format!("{} {}", Colour::Black.bold().paint(timestamp), record.args())
		} else {
			let name = thread::current().name().map_or_else(Default::default, |x| format!("{}", Colour::Blue.bold().paint(x)));
			format!("{} {} {} {}  {}", Colour::Black.bold().paint(timestamp), name, record.level(), record.target(), record.args())
		};

		let removed_color = kill_color(with_color.as_ref());

		let ret = match enable_color {
			true => with_color,
			false => removed_color.clone(),
		};

		if let Some(mut file) = maybe_file.as_ref() {
			// ignore errors - there's nothing we can do
			let _ = file.write_all(removed_color.as_bytes());
			let _ = file.write_all(b"\n");
		}
		logger.append(removed_color);
		if !isatty && record.level() <= LogLevel::Info && atty::is(atty::Stream::Stdout) {
			// duplicate INFO/WARN output to console
			println!("{}", ret);
		}

		ret
    };

	builder.format(format);
	builder.init()
		.and_then(|_| {
			*ROTATING_LOGGER.lock() = Arc::downgrade(&logs);
			Ok(logs)
		})
		// couldn't create new logger - try to fall back on previous logger.
		.or_else(|err| match ROTATING_LOGGER.lock().upgrade() {
			Some(l) => Ok(l),
			// no previous logger. fatal.
			None => Err(format!("{:?}", err)),
		})
}
