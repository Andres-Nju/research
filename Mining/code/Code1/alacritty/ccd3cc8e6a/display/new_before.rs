    pub fn new(
        config: &Config,
        options: &cli::Options,
    ) -> Result<Display, Error> {
        // Extract some properties from config
        let font = config.font();
        let dpi = config.dpi();
        let render_timer = config.render_timer();

        // Create the window where Alacritty will be displayed
        let mut window = Window::new(&options.title)?;

        // get window properties for initializing the other subsytems
        let size = window.inner_size_pixels()
            .expect("glutin returns window size");
        let dpr = window.hidpi_factor();

        info!("device_pixel_ratio: {}", dpr);

        let rasterizer = font::Rasterizer::new(dpi.x(), dpi.y(), dpr, config.use_thin_strokes())?;

        // Create renderer
        let mut renderer = QuadRenderer::new(&config, size)?;

        // Initialize glyph cache
        let glyph_cache = {
            info!("Initializing glyph cache");
            let init_start = ::std::time::Instant::now();

            let cache = renderer.with_loader(|mut api| {
                GlyphCache::new(rasterizer, config, &mut api)
            })?;

            let stop = init_start.elapsed();
            let stop_f = stop.as_secs() as f64 + stop.subsec_nanos() as f64 / 1_000_000_000f64;
            info!("Finished initializing glyph cache in {}", stop_f);

            cache
        };

        // Need font metrics to resize the window properly. This suggests to me the
        // font metrics should be computed before creating the window in the first
        // place so that a resize is not needed.
        let metrics = glyph_cache.font_metrics();
        let cell_width = (metrics.average_advance + font.offset().x as f64) as u32;
        let cell_height = (metrics.line_height + font.offset().y as f64) as u32;

        // Resize window to specified dimensions
        let dimensions = options.dimensions()
            .unwrap_or_else(|| config.dimensions());
        let width = cell_width * dimensions.columns_u32();
        let height = cell_height * dimensions.lines_u32();
        let size = Size { width: Pixels(width), height: Pixels(height) };
        info!("set_inner_size: {}", size);

        let viewport_size = Size {
            width: Pixels(width + 2 * config.padding().x as u32),
            height: Pixels(width + 2 * config.padding().y as u32),
        };
        window.set_inner_size(&viewport_size);
        renderer.resize(viewport_size.width.0 as _, viewport_size.height.0 as _);
        info!("Cell Size: ({} x {})", cell_width, cell_height);

        let size_info = SizeInfo {
            width: viewport_size.width.0 as f32,
            height: viewport_size.height.0 as f32,
            cell_width: cell_width as f32,
            cell_height: cell_height as f32,
            padding_x: config.padding().x.floor(),
            padding_y: config.padding().y.floor(),
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
        renderer.with_api(config, &size_info, 0. /* visual bell intensity */, |api| api.clear());

        let mut display = Display {
            window: window,
            renderer: renderer,
            glyph_cache: glyph_cache,
            render_timer: render_timer,
            tx: tx,
            rx: rx,
            meter: Meter::new(),
            size_info: size_info,
        };

        let resize_tx = display.resize_channel();
        let proxy = display.window.create_window_proxy();
        display.window.set_resize_callback(move |width, height| {
            let _ = resize_tx.send((width, height));
            proxy.wakeup_event_loop();
        });

        Ok(display)
    }
