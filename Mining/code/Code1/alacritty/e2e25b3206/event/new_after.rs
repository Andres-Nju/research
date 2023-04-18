    pub fn new(
        notifier: N,
        resize_tx: mpsc::Sender<PhysicalSize>,
        config: &Config,
        size_info: SizeInfo,
    ) -> Processor<N> {
        Processor {
            key_bindings: config.key_bindings.to_vec(),
            mouse_bindings: config.mouse_bindings.to_vec(),
            mouse_config: config.mouse.to_owned(),
            scrolling_config: config.scrolling,
            print_events: config.debug.print_events,
            wait_for_event: true,
            notifier,
            resize_tx,
            ref_test: config.debug.ref_test,
            mouse: Default::default(),
            size_info,
            hide_mouse_when_typing: config.mouse.hide_when_typing,
            hide_mouse: false,
            received_count: 0,
            suppress_chars: false,
            last_modifiers: Default::default(),
            pending_events: Vec::with_capacity(4),
            window_changes: Default::default(),
            save_to_clipboard: config.selection.save_to_clipboard,
            alt_send_esc: config.alt_send_esc(),
            is_fullscreen: config.window.startup_mode() == StartupMode::Fullscreen,
            #[cfg(target_os = "macos")]
            is_simple_fullscreen: config.window.startup_mode() == StartupMode::SimpleFullscreen,
            #[cfg(not(target_os = "macos"))]
            is_simple_fullscreen: false,
        }
    }
