pub fn from_cmdline_args(args: &[String]) -> ArgumentParsingResult {
    let (app_name, args) = args.split_first().unwrap();

    let mut opts = Options::new();
    opts.optflag("c", "cpu", "CPU painting");
    opts.optflag("g", "gpu", "GPU painting");
    opts.optopt("o", "output", "Output file", "output.png");
    opts.optopt("s", "size", "Size of tiles", "512");
    opts.optopt("", "device-pixel-ratio", "Device pixels per px", "");
    opts.optopt("t", "threads", "Number of paint threads", "1");
    opts.optflagopt("p", "profile", "Time profiler flag and either a TSV output filename \
        OR an interval for output to Stdout (blank for Stdout with interval of 5s)", "10 \
        OR time.tsv");
    opts.optflagopt("", "profiler-trace-path",
                    "Path to dump a self-contained HTML timeline of profiler traces",
                    "");
    opts.optflagopt("m", "memory-profile", "Memory profiler flag and output interval", "10");
    opts.optflag("x", "exit", "Exit after load flag");
    opts.optopt("y", "layout-threads", "Number of threads to use for layout", "1");
    opts.optflag("i", "nonincremental-layout", "Enable to turn off incremental layout.");
    opts.optflagopt("", "userscripts",
                    "Uses userscripts in resources/user-agent-js, or a specified full path", "");
    opts.optmulti("", "user-stylesheet",
                  "A user stylesheet to be added to every document", "file.css");
    opts.optflag("z", "headless", "Headless mode");
    opts.optflag("f", "hard-fail", "Exit on thread failure instead of displaying about:failure");
    opts.optflag("F", "soft-fail", "Display about:failure on thread failure instead of exiting");
    opts.optflagopt("", "remote-debugging-port", "Start remote debugger server on port", "2794");
    opts.optflagopt("", "devtools", "Start remote devtools server on port", "6000");
    opts.optflagopt("", "webdriver", "Start remote WebDriver server on port", "7000");
    opts.optopt("", "resolution", "Set window resolution.", "1024x740");
    opts.optopt("u",
                "user-agent",
                "Set custom user agent string (or android / desktop for platform default)",
                "NCSA Mosaic/1.0 (X11;SunOS 4.1.4 sun4m)");
    opts.optflag("M", "multiprocess", "Run in multiprocess mode");
    opts.optflag("S", "sandbox", "Run in a sandbox if multiprocess");
    opts.optopt("",
                "random-pipeline-closure-probability",
                "Probability of randomly closing a pipeline (for testing constellation hardening).",
                "0.0");
    opts.optopt("", "random-pipeline-closure-seed", "A fixed seed for repeatbility of random pipeline closure.", "");
    opts.optmulti("Z", "debug",
                  "A comma-separated string of debug options. Pass help to show available options.", "");
    opts.optflag("h", "help", "Print this message");
    opts.optopt("", "resources-path", "Path to find static resources", "/home/servo/resources");
    opts.optopt("", "certificate-path", "Path to find SSL certificates", "/home/servo/resources/certs");
    opts.optopt("", "content-process" , "Run as a content process and connect to the given pipe",
                "servo-ipc-channel.abcdefg");
    opts.optmulti("", "pref",
                  "A preference to set to enable", "dom.mozbrowser.enabled");
    opts.optflag("b", "no-native-titlebar", "Do not use native titlebar");
    opts.optflag("w", "webrender", "Use webrender backend");
    opts.optopt("G", "graphics", "Select graphics backend (gl or es2)", "gl");
    opts.optopt("", "config-dir",
                    "config directory following xdg spec on linux platform", "");
    opts.optflag("v", "version", "Display servo version information");
    opts.optflag("", "unminify-js", "Unminify Javascript");

    let opt_match = match opts.parse(args) {
        Ok(m) => m,
        Err(f) => args_fail(&f.to_string()),
    };

    set_resources_path(opt_match.opt_str("resources-path"));

    if opt_match.opt_present("h") || opt_match.opt_present("help") {
        print_usage(app_name, &opts);
        process::exit(0);
    };

    // If this is the content process, we'll receive the real options over IPC. So just fill in
    // some dummy options for now.
    if let Some(content_process) = opt_match.opt_str("content-process") {
        MULTIPROCESS.store(true, Ordering::SeqCst);
        return ArgumentParsingResult::ContentProcess(content_process);
    }

    let mut debug_options = DebugOptions::default();

    for debug_string in opt_match.opt_strs("Z") {
        if let Err(e) = debug_options.extend(debug_string) {
            args_fail(&format!("error: unrecognized debug option: {}", e));
        }
    }

    if debug_options.help {
        print_debug_usage(app_name)
    }

    let cwd = env::current_dir().unwrap();
    let homepage_pref = PREFS.get("shell.homepage");
    let url_opt = if !opt_match.free.is_empty() {
        Some(&opt_match.free[0][..])
    } else {
        homepage_pref.as_string()
    };
    let is_running_problem_test =
        url_opt
        .as_ref()
        .map_or(false, |url|
             url.starts_with("http://web-platform.test:8000/2dcontext/drawing-images-to-the-canvas/") ||
             url.starts_with("http://web-platform.test:8000/_mozilla/mozilla/canvas/") ||
             url.starts_with("http://web-platform.test:8000/_mozilla/css/canvas_over_area.html"));

    let url = match url_opt {
        Some(url_string) => {
            parse_url_or_filename(&cwd, url_string)
                .unwrap_or_else(|()| args_fail("URL parsing failed"))
        },
        None => {
            print_usage(app_name, &opts);
            args_fail("servo asks that you provide a URL")
        }
    };

    let tile_size: usize = match opt_match.opt_str("s") {
        Some(tile_size_str) => tile_size_str.parse()
            .unwrap_or_else(|err| args_fail(&format!("Error parsing option: -s ({})", err))),
        None => 512,
    };

    let device_pixels_per_px = opt_match.opt_str("device-pixel-ratio").map(|dppx_str|
        dppx_str.parse()
            .unwrap_or_else(|err| args_fail(&format!("Error parsing option: --device-pixel-ratio ({})", err)))
    );

    // If only the flag is present, default to a 5 second period for both profilers
    let time_profiling = if opt_match.opt_present("p") {
        match opt_match.opt_str("p") {
            Some(argument) => match argument.parse::<f64>() {
                Ok(interval) => Some(OutputOptions::Stdout(interval)) ,
                Err(_) => Some(OutputOptions::FileName(argument)),
            },
            None => Some(OutputOptions::Stdout(5.0 as f64)),
        }
    } else {
        // if the p option doesn't exist:
        None
    };

    if let Some(ref time_profiler_trace_path) = opt_match.opt_str("profiler-trace-path") {
        let mut path = PathBuf::from(time_profiler_trace_path);
        path.pop();
        if let Err(why) = fs::create_dir_all(&path) {
            error!("Couldn't create/open {:?}: {:?}",
                Path::new(time_profiler_trace_path).to_string_lossy(), why);
        }
    }

    let mem_profiler_period = opt_match.opt_default("m", "5").map(|period| {
        period.parse().unwrap_or_else(|err| args_fail(&format!("Error parsing option: -m ({})", err)))
    });

    let mut layout_threads: Option<usize> = opt_match.opt_str("y")
        .map(|layout_threads_str| {
            layout_threads_str.parse()
                .unwrap_or_else(|err| args_fail(&format!("Error parsing option: -y ({})", err)))
        });

    let nonincremental_layout = opt_match.opt_present("i");

    let random_pipeline_closure_probability = opt_match.opt_str("random-pipeline-closure-probability").map(|prob|
        prob.parse().unwrap_or_else(|err| {
            args_fail(&format!("Error parsing option: --random-pipeline-closure-probability ({})", err))
        })
    );

    let random_pipeline_closure_seed = opt_match.opt_str("random-pipeline-closure-seed").map(|seed|
        seed.parse().unwrap_or_else(|err| {
            args_fail(&format!("Error parsing option: --random-pipeline-closure-seed ({})", err))
        })
    );

    let mut bubble_inline_sizes_separately = debug_options.bubble_widths;
    if debug_options.trace_layout {
        layout_threads = Some(1);
        bubble_inline_sizes_separately = true;
    }

    let debugger_port = opt_match.opt_default("remote-debugging-port", "2794").map(|port| {
        port.parse()
            .unwrap_or_else(|err| args_fail(&format!("Error parsing option: --remote-debugging-port ({})", err)))
    });

    let devtools_port = opt_match.opt_default("devtools", "6000").map(|port| {
        port.parse().unwrap_or_else(|err| args_fail(&format!("Error parsing option: --devtools ({})", err)))
    });

    let webdriver_port = opt_match.opt_default("webdriver", "7000").map(|port| {
        port.parse().unwrap_or_else(|err| args_fail(&format!("Error parsing option: --webdriver ({})", err)))
    });

    let initial_window_size = match opt_match.opt_str("resolution") {
        Some(res_string) => {
            let res: Vec<u32> = res_string.split('x').map(|r| {
                r.parse().unwrap_or_else(|err| args_fail(&format!("Error parsing option: --resolution ({})", err)))
            }).collect();
            TypedSize2D::new(res[0], res[1])
        }
        None => {
            TypedSize2D::new(1024, 740)
        }
    };

    if opt_match.opt_present("M") {
        MULTIPROCESS.store(true, Ordering::SeqCst)
    }

    let user_agent = match opt_match.opt_str("u") {
        Some(ref ua) if ua == "android" => default_user_agent_string(UserAgent::Android).into(),
        Some(ref ua) if ua == "desktop" => default_user_agent_string(UserAgent::Desktop).into(),
        Some(ua) => ua.into(),
        None => default_user_agent_string(DEFAULT_USER_AGENT).into(),
    };

    let user_stylesheets = opt_match.opt_strs("user-stylesheet").iter().map(|filename| {
        let path = cwd.join(filename);
        let url = ServoUrl::from_url(Url::from_file_path(&path).unwrap());
        let mut contents = Vec::new();
        File::open(path)
            .unwrap_or_else(|err| args_fail(&format!("Couldn't open {}: {}", filename, err)))
            .read_to_end(&mut contents)
            .unwrap_or_else(|err| args_fail(&format!("Couldn't read {}: {}", filename, err)));
        (contents, url)
    }).collect();

    let do_not_use_native_titlebar =
        opt_match.opt_present("b") ||
        !PREFS.get("shell.native-titlebar.enabled").as_boolean().unwrap();

    let is_printing_version = opt_match.opt_present("v") || opt_match.opt_present("version");

    let opts = Opts {
        is_running_problem_test: is_running_problem_test,
        url: Some(url),
        tile_size: tile_size,
        device_pixels_per_px: device_pixels_per_px,
        time_profiling: time_profiling,
        time_profiler_trace_path: opt_match.opt_str("profiler-trace-path"),
        mem_profiler_period: mem_profiler_period,
        nonincremental_layout: nonincremental_layout,
        userscripts: opt_match.opt_default("userscripts", ""),
        user_stylesheets: user_stylesheets,
        output_file: opt_match.opt_str("o"),
        replace_surrogates: debug_options.replace_surrogates,
        gc_profile: debug_options.gc_profile,
        load_webfonts_synchronously: debug_options.load_webfonts_synchronously,
        headless: opt_match.opt_present("z"),
        hard_fail: opt_match.opt_present("f") && !opt_match.opt_present("F"),
        bubble_inline_sizes_separately: bubble_inline_sizes_separately,
        profile_script_events: debug_options.profile_script_events,
        profile_heartbeats: debug_options.profile_heartbeats,
        trace_layout: debug_options.trace_layout,
        debugger_port: debugger_port,
        devtools_port: devtools_port,
        webdriver_port: webdriver_port,
        initial_window_size: initial_window_size,
        user_agent: user_agent,
        multiprocess: opt_match.opt_present("M"),
        sandbox: opt_match.opt_present("S"),
        random_pipeline_closure_probability: random_pipeline_closure_probability,
        random_pipeline_closure_seed: random_pipeline_closure_seed,
        show_debug_fragment_borders: debug_options.show_fragment_borders,
        show_debug_parallel_layout: debug_options.show_parallel_layout,
        enable_text_antialiasing: !debug_options.disable_text_aa,
        enable_subpixel_text_antialiasing: !debug_options.disable_subpixel_aa,
        enable_canvas_antialiasing: !debug_options.disable_canvas_aa,
        dump_style_tree: debug_options.dump_style_tree,
        dump_rule_tree: debug_options.dump_rule_tree,
        dump_flow_tree: debug_options.dump_flow_tree,
        dump_display_list: debug_options.dump_display_list,
        dump_display_list_json: debug_options.dump_display_list_json,
        relayout_event: debug_options.relayout_event,
        disable_share_style_cache: debug_options.disable_share_style_cache,
        style_sharing_stats: debug_options.style_sharing_stats,
        convert_mouse_to_touch: debug_options.convert_mouse_to_touch,
        exit_after_load: opt_match.opt_present("x"),
        no_native_titlebar: do_not_use_native_titlebar,
        enable_vsync: !debug_options.disable_vsync,
        webrender_stats: debug_options.webrender_stats,
        use_msaa: debug_options.use_msaa,
        config_dir: opt_match.opt_str("config-dir").map(Into::into),
        full_backtraces: debug_options.full_backtraces,
        is_printing_version: is_printing_version,
        webrender_debug: debug_options.webrender_debug,
        webrender_record: debug_options.webrender_record,
        webrender_batch: !debug_options.webrender_disable_batch,
        precache_shaders: debug_options.precache_shaders,
        signpost: debug_options.signpost,
        certificate_path: opt_match.opt_str("certificate-path"),
        unminify_js: opt_match.opt_present("unminify-js"),
    };

    set_defaults(opts);

    // These must happen after setting the default options, since the prefs rely on
    // on the resource path.
    // Note that command line preferences have the highest precedence

    prefs::add_user_prefs();

    for pref in opt_match.opt_strs("pref").iter() {
        parse_pref_from_command_line(pref);
    }

    if let Some(layout_threads) = layout_threads {
        PREFS.set("layout.threads", PrefValue::Number(layout_threads as f64));
    } else if let Some(layout_threads) = PREFS.get("layout.threads").as_string() {
        PREFS.set("layout.threads", PrefValue::Number(layout_threads.parse::<f64>().unwrap()));
    } else if *PREFS.get("layout.threads") == PrefValue::Missing {
        let layout_threads = cmp::max(num_cpus::get() * 3 / 4, 1);
        PREFS.set("layout.threads", PrefValue::Number(layout_threads as f64));
    }

    ArgumentParsingResult::ChromeProcess
}
