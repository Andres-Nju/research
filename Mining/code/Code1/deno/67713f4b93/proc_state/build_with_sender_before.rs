  async fn build_with_sender(
    cli_options: Arc<CliOptions>,
    maybe_sender: Option<tokio::sync::mpsc::UnboundedSender<Vec<PathBuf>>>,
  ) -> Result<Self, AnyError> {
    let blob_store = BlobStore::default();
    let broadcast_channel = InMemoryBroadcastChannel::default();
    let shared_array_buffer_store = SharedArrayBufferStore::default();
    let compiled_wasm_module_store = CompiledWasmModuleStore::default();
    let dir = cli_options.resolve_deno_dir()?;
    let deps_cache_location = dir.root.join("deps");
    let http_cache = http_cache::HttpCache::new(&deps_cache_location);
    let root_cert_store = cli_options.resolve_root_cert_store()?;
    let cache_usage = cli_options.cache_setting();
    let file_fetcher = FileFetcher::new(
      http_cache,
      cache_usage,
      !cli_options.no_remote(),
      Some(root_cert_store.clone()),
      blob_store.clone(),
      cli_options
        .unsafely_ignore_certificate_errors()
        .map(ToOwned::to_owned),
    )?;

    let lockfile = cli_options
      .resolve_lock_file()?
      .map(|f| Arc::new(Mutex::new(f)));
    let maybe_import_map_specifier =
      cli_options.resolve_import_map_specifier()?;

    let maybe_import_map =
      if let Some(import_map_specifier) = maybe_import_map_specifier {
        let file = file_fetcher
          .fetch(&import_map_specifier, &mut Permissions::allow_all())
          .await
          .context(format!(
            "Unable to load '{}' import map",
            import_map_specifier
          ))?;
        let import_map =
          import_map_from_text(&import_map_specifier, &file.source)?;
        Some(Arc::new(import_map))
      } else {
        None
      };

    let maybe_inspector_server =
      cli_options.resolve_inspector_server().map(Arc::new);

    // FIXME(bartlomieju): `NodeEsmResolver` is not aware of JSX resolver
    // created below
    let node_resolver = NodeEsmResolver::new(
      maybe_import_map.clone().map(ImportMapResolver::new),
    );
    let maybe_import_map_resolver =
      maybe_import_map.clone().map(ImportMapResolver::new);
    let maybe_jsx_resolver = cli_options
      .to_maybe_jsx_import_source_config()
      .map(|cfg| JsxResolver::new(cfg, maybe_import_map_resolver.clone()));
    let maybe_resolver: Option<
      Arc<dyn deno_graph::source::Resolver + Send + Sync>,
    > = if cli_options.compat() {
      Some(Arc::new(node_resolver))
    } else if let Some(jsx_resolver) = maybe_jsx_resolver {
      // the JSX resolver offloads to the import map if present, otherwise uses
      // the default Deno explicit import resolution.
      Some(Arc::new(jsx_resolver))
    } else if let Some(import_map_resolver) = maybe_import_map_resolver {
      Some(Arc::new(import_map_resolver))
    } else {
      None
    };

    let maybe_file_watcher_reporter =
      maybe_sender.map(|sender| FileWatcherReporter {
        sender,
        file_paths: Arc::new(Mutex::new(vec![])),
      });

    let ts_config_result =
      cli_options.resolve_ts_config_for_emit(TsConfigType::Emit)?;
    if let Some(ignored_options) = ts_config_result.maybe_ignored_options {
      warn!("{}", ignored_options);
    }
    let emit_cache = EmitCache::new(dir.gen_cache.clone());
    let parsed_source_cache =
      ParsedSourceCache::new(Some(dir.dep_analysis_db_file_path()));
    let npm_resolver = GlobalNpmPackageResolver::from_deno_dir(
      &dir,
      cli_options.reload_flag(),
      cli_options.cache_setting(),
      cli_options.unstable(),
    )?;

    Ok(ProcState(Arc::new(Inner {
      dir,
      options: cli_options,
      emit_cache,
      emit_options_hash: FastInsecureHasher::new()
        // todo(dsherret): use hash of emit options instead as it's more specific
        .write(&ts_config_result.ts_config.as_bytes())
        .finish(),
      emit_options: ts_config_result.ts_config.into(),
      file_fetcher,
      graph_data: Default::default(),
      lockfile,
      maybe_import_map,
      maybe_inspector_server,
      root_cert_store,
      blob_store,
      broadcast_channel,
      shared_array_buffer_store,
      compiled_wasm_module_store,
      parsed_source_cache,
      maybe_resolver,
      maybe_file_watcher_reporter,
      npm_resolver,
      cjs_resolutions: Default::default(),
    })))
  }
