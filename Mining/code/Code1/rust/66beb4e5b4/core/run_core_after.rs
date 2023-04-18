pub fn run_core(search_paths: SearchPaths,
                cfgs: Vec<String>,
                externs: config::Externs,
                input: Input,
                triple: Option<TargetTriple>,
                maybe_sysroot: Option<PathBuf>,
                allow_warnings: bool,
                crate_name: Option<String>,
                force_unstable_if_unmarked: bool,
                edition: Edition,
                cg: CodegenOptions,
                error_format: ErrorOutputType) -> (clean::Crate, RenderInfo)
{
    // Parse, resolve, and typecheck the given crate.

    let cpath = match input {
        Input::File(ref p) => Some(p.clone()),
        _ => None
    };

    let intra_link_resolution_failure_name = lint::builtin::INTRA_DOC_LINK_RESOLUTION_FAILURE.name;
    let warnings_lint_name = lint::builtin::WARNINGS.name;
    let missing_docs = rustc_lint::builtin::MISSING_DOCS.name;
    let lints = lint::builtin::HardwiredLints.get_lints()
                    .into_iter()
                    .chain(rustc_lint::SoftLints.get_lints().into_iter())
                    .filter_map(|lint| {
                        if lint.name == warnings_lint_name ||
                           lint.name == intra_link_resolution_failure_name {
                            None
                        } else {
                            Some((lint.name_lower(), lint::Allow))
                        }
                    })
                    .collect::<Vec<_>>();

    let host_triple = TargetTriple::from_triple(config::host_triple());
    // plays with error output here!
    let sessopts = config::Options {
        maybe_sysroot,
        search_paths,
        crate_types: vec![config::CrateTypeRlib],
        lint_opts: if !allow_warnings {
            lints
        } else {
            vec![]
        },
        lint_cap: Some(lint::Forbid),
        cg,
        externs,
        target_triple: triple.unwrap_or(host_triple),
        // Ensure that rustdoc works even if rustc is feature-staged
        unstable_features: UnstableFeatures::Allow,
        actually_rustdoc: true,
        debugging_opts: config::DebuggingOptions {
            force_unstable_if_unmarked,
            ..config::basic_debugging_options()
        },
        error_format,
        edition,
        ..config::basic_options()
    };
    driver::spawn_thread_pool(sessopts, move |sessopts| {
        let codemap = Lrc::new(codemap::CodeMap::new(sessopts.file_path_mapping()));
        let diagnostic_handler = new_handler(error_format, Some(codemap.clone()));

        let mut sess = session::build_session_(
            sessopts, cpath, diagnostic_handler, codemap,
        );

        lint::builtin::HardwiredLints.get_lints()
                                     .into_iter()
                                     .chain(rustc_lint::SoftLints.get_lints().into_iter())
                                     .filter_map(|lint| {
                                         // We don't want to whitelist *all* lints so let's
                                         // ignore those ones.
                                         if lint.name == warnings_lint_name ||
                                            lint.name == intra_link_resolution_failure_name ||
                                            lint.name == missing_docs {
                                             None
                                         } else {
                                             Some(lint)
                                         }
                                     })
                                     .for_each(|l| {
                                         sess.driver_lint_caps.insert(lint::LintId::of(l),
                                                                      lint::Allow);
                                     });

        let codegen_backend = rustc_driver::get_codegen_backend(&sess);
        let cstore = Rc::new(CStore::new(codegen_backend.metadata_loader()));
        rustc_lint::register_builtins(&mut sess.lint_store.borrow_mut(), Some(&sess));

        let mut cfg = config::build_configuration(&sess, config::parse_cfgspecs(cfgs));
        target_features::add_configuration(&mut cfg, &sess, &*codegen_backend);
        sess.parse_sess.config = cfg;

        let control = &driver::CompileController::basic();

        let krate = panictry!(driver::phase_1_parse_input(control, &sess, &input));

        let name = match crate_name {
            Some(ref crate_name) => crate_name.clone(),
            None => ::rustc_codegen_utils::link::find_crate_name(Some(&sess), &krate.attrs, &input),
        };

        let mut crate_loader = CrateLoader::new(&sess, &cstore, &name);

        let resolver_arenas = resolve::Resolver::arenas();
        let result = driver::phase_2_configure_and_expand_inner(&sess,
                                                        &cstore,
                                                        krate,
                                                        None,
                                                        &name,
                                                        None,
                                                        resolve::MakeGlobMap::No,
                                                        &resolver_arenas,
                                                        &mut crate_loader,
                                                        |_| Ok(()));
        let driver::InnerExpansionResult {
            mut hir_forest,
            mut resolver,
            ..
        } = abort_on_err(result, &sess);

        resolver.ignore_extern_prelude_feature = true;

        // We need to hold on to the complete resolver, so we clone everything
        // for the analysis passes to use. Suboptimal, but necessary in the
        // current architecture.
        let defs = resolver.definitions.clone();
        let resolutions = ty::Resolutions {
            freevars: resolver.freevars.clone(),
            export_map: resolver.export_map.clone(),
            trait_map: resolver.trait_map.clone(),
            maybe_unused_trait_imports: resolver.maybe_unused_trait_imports.clone(),
            maybe_unused_extern_crates: resolver.maybe_unused_extern_crates.clone(),
        };
        let analysis = ty::CrateAnalysis {
            access_levels: Lrc::new(AccessLevels::default()),
            name: name.to_string(),
            glob_map: if resolver.make_glob_map { Some(resolver.glob_map.clone()) } else { None },
        };

        let arenas = AllArenas::new();
        let hir_map = hir_map::map_crate(&sess, &*cstore, &mut hir_forest, &defs);
        let output_filenames = driver::build_output_filenames(&input,
                                                            &None,
                                                            &None,
                                                            &[],
                                                            &sess);

        let resolver = RefCell::new(resolver);
        abort_on_err(driver::phase_3_run_analysis_passes(&*codegen_backend,
                                                        control,
                                                        &sess,
                                                        &*cstore,
                                                        hir_map,
                                                        analysis,
                                                        resolutions,
                                                        &arenas,
                                                        &name,
                                                        &output_filenames,
                                                        |tcx, analysis, _, result| {
            if let Err(_) = result {
                sess.fatal("Compilation failed, aborting rustdoc");
            }

            let ty::CrateAnalysis { access_levels, .. } = analysis;

            // Convert from a NodeId set to a DefId set since we don't always have easy access
            // to the map from defid -> nodeid
            let access_levels = AccessLevels {
                map: access_levels.map.iter()
                                    .map(|(&k, &v)| (tcx.hir.local_def_id(k), v))
                                    .collect()
            };

            let send_trait = if crate_name == Some("core".to_string()) {
                clean::get_trait_def_id(&tcx, &["marker", "Send"], true)
            } else {
                clean::get_trait_def_id(&tcx, &["core", "marker", "Send"], false)
            };

            let ctxt = DocContext {
                tcx,
                resolver: &resolver,
                crate_name,
                cstore: cstore.clone(),
                populated_all_crate_impls: Cell::new(false),
                access_levels: RefCell::new(access_levels),
                external_traits: Default::default(),
                active_extern_traits: Default::default(),
                renderinfo: Default::default(),
                ty_substs: Default::default(),
                lt_substs: Default::default(),
                impl_trait_bounds: Default::default(),
                mod_ids: Default::default(),
                send_trait: send_trait,
                fake_def_ids: RefCell::new(FxHashMap()),
                all_fake_def_ids: RefCell::new(FxHashSet()),
                generated_synthetics: RefCell::new(FxHashSet()),
            };
            debug!("crate: {:?}", tcx.hir.krate());

            let krate = {
                let mut v = RustdocVisitor::new(&*cstore, &ctxt);
                v.visit(tcx.hir.krate());
                v.clean(&ctxt)
            };

            (krate, ctxt.renderinfo.into_inner())
        }), &sess)
    })
}
