  pub fn run(mut self, context: Context<A>) -> crate::Result<()> {
    let manager = WindowManager::with_handlers(
      context,
      self.plugins,
      self.invoke_handler,
      self.on_page_load,
      self.custom_protocols,
    );

    // set up all the windows defined in the config
    for config in manager.config().tauri.windows.clone() {
      let url = config.url.clone();
      let label = config
        .label
        .parse()
        .unwrap_or_else(|_| panic!("bad label found in config: {}", config.label));

      self
        .pending_windows
        .push(PendingWindow::with_config(config, label, url));
    }

    manager.initialize_plugins()?;

    let mut app = App {
      runtime: R::new()?,
      manager,
    };

    let pending_labels = self
      .pending_windows
      .iter()
      .map(|p| p.label.clone())
      .collect::<Vec<_>>();

    #[cfg(feature = "updater")]
    let mut main_window = None;

    for pending in self.pending_windows {
      let pending = app.manager.prepare_window(pending, &pending_labels)?;
      let detached = app.runtime.create_window(pending)?;
      let _window = app.manager.attach_window(detached);
      #[cfg(feature = "updater")]
      if main_window.is_none() {
        main_window = Some(_window);
      }
    }

    #[cfg(feature = "updater")]
    app.run_updater(main_window);

    (self.setup)(&mut app).map_err(|e| crate::Error::Setup(e.to_string()))?;
    app.runtime.run();
    Ok(())
  }
