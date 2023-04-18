pub extern "C" fn cef_initialize(args: *const cef_main_args_t,
                                 settings: *mut cef_settings_t,
                                 application: *mut cef_app_t,
                                 _windows_sandbox_info: *const c_void)
                                 -> c_int {
    if args.is_null() {
        return 0;
    }
    unsafe {
        if !CEF_APP.is_null() {
            panic!("Attempting to call cef_initialize() multiple times!");
        }
    }

    unsafe {
        command_line_init((*args).argc, (*args).argv);

        if !application.is_null() {
            (*application).get_browser_process_handler.map(|cb| {
                    let handler = cb(application);
                    if !handler.is_null() {
                        (*handler).on_context_initialized.map(|hcb| hcb(handler));
                    }
            });
            CEF_APP = application;
        }
    }

    let rendering_threads = unsafe {
        if ((*settings).rendering_threads as usize) < 1 {
            1
        } else if (*settings).rendering_threads as usize > MAX_RENDERING_THREADS {
            MAX_RENDERING_THREADS
        } else {
            (*settings).rendering_threads as usize
        }
    };

    let mut temp_opts = opts::default_opts();
    temp_opts.paint_threads = rendering_threads;
    temp_opts.layout_threads = rendering_threads;
    temp_opts.headless = false;
    temp_opts.hard_fail = false;
    temp_opts.enable_text_antialiasing = true;
    temp_opts.enable_canvas_antialiasing = true;
    temp_opts.url = None;
    opts::set_defaults(temp_opts);

    if unsafe { (*settings).windowless_rendering_enabled != 0 } {
        init_window();
    }

    return 1
}
