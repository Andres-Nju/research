    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Error::Window(ref err) => err.fmt(f),
            Error::Font(ref err) => err.fmt(f),
            Error::Render(ref err) => err.fmt(f),
        }
    }
}

impl From<window::Error> for Error {
    fn from(val: window::Error) -> Error {
        Error::Window(val)
    }
}

impl From<font::Error> for Error {
    fn from(val: font::Error) -> Error {
        Error::Font(val)
    }
}

impl From<renderer::Error> for Error {
    fn from(val: renderer::Error) -> Error {
        Error::Render(val)
    }
}

/// The display wraps a window, font rasterizer, and GPU renderer
pub struct Display {
    window: Window,
    renderer: QuadRenderer,
    glyph_cache: GlyphCache,
    render_timer: bool,
    rx: mpsc::Receiver<PhysicalSize>,
    tx: mpsc::Sender<PhysicalSize>,
    meter: Meter,
    font_size: font::Size,
    size_info: SizeInfo,
    last_message: Option<Message>,
}

/// Can wakeup the render loop from other threads
pub struct Notifier(window::Proxy);

/// Types that are interested in when the display is resized
pub trait OnResize {
    fn on_resize(&mut self, size: &SizeInfo);
}

impl Notifier {
    pub fn notify(&self) {
        self.0.wakeup_event_loop();
    }
}

impl Display {
    pub fn notifier(&self) -> Notifier {
        Notifier(self.window.create_window_proxy())
    }

    pub fn update_config(&mut self, config: &Config) {
        self.render_timer = config.render_timer();
    }

    /// Get size info about the display
    pub fn size(&self) -> &SizeInfo {
        &self.size_info
    }

    pub fn new(config: &Config, options: &cli::Options) -> Result<Display, Error> {
        // Extract some properties from config
        let render_timer = config.render_timer();

        // Guess DPR based on first monitor
        let event_loop = EventsLoop::new();
        let estimated_dpr =
            event_loop.get_available_monitors().next().map(|m| m.get_hidpi_factor()).unwrap_or(1.);

        // Guess the target window dimensions
        let metrics = GlyphCache::static_metrics(config, estimated_dpr as f32)?;
        let (cell_width, cell_height) = Self::compute_cell_size(config, &metrics);
        let dimensions =
            Self::calculate_dimensions(config, options, estimated_dpr, cell_width, cell_height);

        debug!("Estimated DPR: {}", estimated_dpr);
        debug!("Estimated Cell Size: {} x {}", cell_width, cell_height);
        debug!("Estimated Dimensions: {:?}", dimensions);

        // Create the window where Alacritty will be displayed
        let logical = dimensions.map(|d| PhysicalSize::new(d.0, d.1).to_logical(estimated_dpr));
        let mut window = Window::new(event_loop, &options, config.window(), logical)?;

        let dpr = window.hidpi_factor();
        info!("Device pixel ratio: {}", dpr);

        // get window properties for initializing the other subsystems
        let mut viewport_size =
            window.inner_size_pixels().expect("glutin returns window size").to_physical(dpr);

        // Create renderer
        let mut renderer = QuadRenderer::new()?;

        let (glyph_cache, cell_width, cell_height) =
            Self::new_glyph_cache(dpr, &mut renderer, config)?;

        let mut padding_x = f64::from(config.padding().x) * dpr;
        let mut padding_y = f64::from(config.padding().y) * dpr;

        if let Some((width, height)) =
            Self::calculate_dimensions(config, options, dpr, cell_width, cell_height)
        {
            if dimensions == Some((width, height)) {
                info!("Estimated DPR correctly, skipping resize");
            } else {
                viewport_size = PhysicalSize::new(width, height);
                window.set_inner_size(viewport_size.to_logical(dpr));
            }
        } else if config.window().dynamic_padding() {
            // Make sure additional padding is spread evenly
            let cw = f64::from(cell_width);
            let ch = f64::from(cell_height);
            padding_x = padding_x + (viewport_size.width - 2. * padding_x) % cw / 2.;
            padding_y = padding_y + (viewport_size.height - 2. * padding_y) % ch / 2.;
        }

        padding_x = padding_x.floor();
        padding_y = padding_y.floor();

        // Update OpenGL projection
        renderer.resize(viewport_size, padding_x as f32, padding_y as f32);

        info!("Cell Size: {} x {}", cell_width, cell_height);
        info!("Padding: {} x {}", padding_x, padding_y);

        let size_info = SizeInfo {
            dpr,
            width: viewport_size.width as f32,
            height: viewport_size.height as f32,
            cell_width: cell_width as f32,
            cell_height: cell_height as f32,
            padding_x: padding_x as f32,
            padding_y: padding_y as f32,
        };

        // Channel for resize events
        //
        // macOS has a callback for getting resize events, the channel is used
        // to queue resize events until the next draw call. Unfortunately, it
        // seems that the event loop is blocked until the window is done
        // resizing. If any drawing were to happen during a resize, it would
        // need to be in the callback.
        let (tx, rx) = mpsc::channel();

        // Clear screen
        let background_color = config.colors().primary.background;
        renderer.with_api(config, &size_info, |api| {
            api.clear(background_color);
        });

        Ok(Display {
            window,
            renderer,
            glyph_cache,
            render_timer,
            tx,
            rx,
            meter: Meter::new(),
            font_size: config.font().size(),
            size_info,
            last_message: None,
        })
    }

    fn calculate_dimensions(
        config: &Config,
        options: &cli::Options,
        dpr: f64,
        cell_width: f32,
        cell_height: f32,
    ) -> Option<(f64, f64)> {
        let dimensions = options.dimensions().unwrap_or_else(|| config.dimensions());

        if dimensions.columns_u32() == 0
            || dimensions.lines_u32() == 0
            || config.window().startup_mode() != StartupMode::Windowed
        {
            return None;
        }

        let padding_x = f64::from(config.padding().x) * dpr;
        let padding_y = f64::from(config.padding().y) * dpr;

        // Calculate new size based on cols/lines specified in config
        let grid_width = cell_width as u32 * dimensions.columns_u32();
        let grid_height = cell_height as u32 * dimensions.lines_u32();

        let width = (f64::from(grid_width) + 2. * padding_x).floor();
        let height = (f64::from(grid_height) + 2. * padding_y).floor();

        Some((width, height))
    }

    fn new_glyph_cache(
        dpr: f64,
        renderer: &mut QuadRenderer,
        config: &Config,
    ) -> Result<(GlyphCache, f32, f32), Error> {
        let font = config.font().clone();
        let rasterizer = font::Rasterizer::new(dpr as f32, config.use_thin_strokes())?;

        // Initialize glyph cache
        let glyph_cache = {
            info!("Initializing glyph cache...");
            let init_start = ::std::time::Instant::now();

            let cache =
                renderer.with_loader(|mut api| GlyphCache::new(rasterizer, &font, &mut api))?;

            let stop = init_start.elapsed();
            let stop_f = stop.as_secs() as f64 + f64::from(stop.subsec_nanos()) / 1_000_000_000f64;
            info!("... finished initializing glyph cache in {}s", stop_f);

            cache
        };

        // Need font metrics to resize the window properly. This suggests to me the
        // font metrics should be computed before creating the window in the first
        // place so that a resize is not needed.
        let (cw, ch) = Self::compute_cell_size(config, &glyph_cache.font_metrics());

        Ok((glyph_cache, cw, ch))
    }

    pub fn update_glyph_cache(&mut self, config: &Config) {
        let cache = &mut self.glyph_cache;
        let dpr = self.size_info.dpr;
        let size = self.font_size;

        self.renderer.with_loader(|mut api| {
            let _ = cache.update_font_size(config.font(), size, dpr, &mut api);
        });

        let (cw, ch) = Self::compute_cell_size(config, &cache.font_metrics());
        self.size_info.cell_width = cw;
        self.size_info.cell_height = ch;
    }

    fn compute_cell_size(config: &Config, metrics: &font::Metrics) -> (f32, f32) {
        let offset_x = f64::from(config.font().offset().x);
        let offset_y = f64::from(config.font().offset().y);
        (
            f32::max(1., ((metrics.average_advance + offset_x) as f32).floor()),
            f32::max(1., ((metrics.line_height + offset_y) as f32).floor()),
        )
    }

    #[inline]
    pub fn resize_channel(&self) -> mpsc::Sender<PhysicalSize> {
        self.tx.clone()
    }

    pub fn window(&mut self) -> &mut Window {
        &mut self.window
    }

    /// Process pending resize events
    pub fn handle_resize(
        &mut self,
        terminal: &mut MutexGuard<'_, Term>,
        config: &Config,
        pty_resize_handle: &mut dyn OnResize,
        processor_resize_handle: &mut dyn OnResize,
    ) {
        let previous_cols = self.size_info.cols();
        let previous_lines = self.size_info.lines();

        // Resize events new_size and are handled outside the poll_events
        // iterator. This has the effect of coalescing multiple resize
        // events into one.
        let mut new_size = None;

        // Take most recent resize event, if any
        while let Ok(size) = self.rx.try_recv() {
            new_size = Some(size);
        }

        // Update the DPR
        let dpr = self.window.hidpi_factor();

        // Font size/DPI factor modification detected
        let font_changed =
            terminal.font_size != self.font_size || (dpr - self.size_info.dpr).abs() > f64::EPSILON;

        // Skip resize if nothing changed
        if let Some(new_size) = new_size {
            if !font_changed
                && (new_size.width - f64::from(self.size_info.width)).abs() < f64::EPSILON
                && (new_size.height - f64::from(self.size_info.height)).abs() < f64::EPSILON
            {
                return;
            }
        }

        if font_changed || self.last_message != terminal.message_buffer_mut().message() {
            if new_size == None {
                // Force a resize to refresh things
                new_size = Some(PhysicalSize::new(
                    f64::from(self.size_info.width) / self.size_info.dpr * dpr,
                    f64::from(self.size_info.height) / self.size_info.dpr * dpr,
                ));
            }

            self.font_size = terminal.font_size;
            self.last_message = terminal.message_buffer_mut().message();
            self.size_info.dpr = dpr;
        }

        if font_changed {
            self.update_glyph_cache(config);
        }

        if let Some(psize) = new_size.take() {
            let width = psize.width as f32;
            let height = psize.height as f32;
            let cell_width = self.size_info.cell_width;
            let cell_height = self.size_info.cell_height;

            self.size_info.width = width;
            self.size_info.height = height;

            let mut padding_x = f32::from(config.padding().x) * dpr as f32;
            let mut padding_y = f32::from(config.padding().y) * dpr as f32;

            if config.window().dynamic_padding() {
                padding_x = padding_x + ((width - 2. * padding_x) % cell_width) / 2.;
                padding_y = padding_y + ((height - 2. * padding_y) % cell_height) / 2.;
            }

            self.size_info.padding_x = padding_x.floor();
            self.size_info.padding_y = padding_y.floor();

            let size = &self.size_info;
            terminal.resize(size);
            processor_resize_handle.on_resize(size);

            // Subtract message bar lines for pty size
            let mut pty_size = *size;
            if let Some(message) = terminal.message_buffer_mut().message() {
                pty_size.height -= pty_size.cell_height * message.text(&size).len() as f32;
            }

            if previous_cols != size.cols() || previous_lines != size.lines() {
                pty_resize_handle.on_resize(&pty_size);
            }

            self.window.resize(psize);
            self.renderer.resize(psize, self.size_info.padding_x, self.size_info.padding_y);
        }
    }
