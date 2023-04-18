pub fn compile_input(
    codegen_backend: Box<dyn CodegenBackend>,
    sess: &Session,
    cstore: &CStore,
    input_path: &Option<PathBuf>,
    input: &Input,
    outdir: &Option<PathBuf>,
    output: &Option<PathBuf>,
    addl_plugins: Option<Vec<String>>,
    control: &CompileController,
) -> CompileResult {
    macro_rules! controller_entry_point {
        ($point: ident, $tsess: expr, $make_state: expr, $phase_result: expr) => {{
            let state = &mut $make_state;
            let phase_result: &CompileResult = &$phase_result;
            if phase_result.is_ok() || control.$point.run_callback_on_error {
                (control.$point.callback)(state);
            }

            if control.$point.stop == Compilation::Stop {
                // FIXME: shouldn't this return Err(CompileIncomplete::Stopped)
                // if there are no errors?
                return $tsess.compile_status();
            }
        }}
    }

    if sess.profile_queries() {
        profile::begin(sess);
    }

    // We need nested scopes here, because the intermediate results can keep
    // large chunks of memory alive and we want to free them as soon as
    // possible to keep the peak memory usage low
    let (outputs, ongoing_codegen, dep_graph) = {
        let krate = match phase_1_parse_input(control, sess, input) {
            Ok(krate) => krate,
            Err(mut parse_error) => {
                parse_error.emit();
                return Err(CompileIncomplete::Errored(ErrorReported));
            }
        };

        let (krate, registry) = {
            let mut compile_state =
                CompileState::state_after_parse(input, sess, outdir, output, krate, &cstore);
            controller_entry_point!(after_parse, sess, compile_state, Ok(()));

            (compile_state.krate.unwrap(), compile_state.registry)
        };

        let outputs = build_output_filenames(input, outdir, output, &krate.attrs, sess);
        let crate_name =
            ::rustc_codegen_utils::link::find_crate_name(Some(sess), &krate.attrs, input);
        install_panic_hook();

        let ExpansionResult {
            expanded_crate,
            defs,
            resolutions,
            mut hir_forest,
        } = {
            phase_2_configure_and_expand(
                sess,
                &cstore,
                krate,
                registry,
                &crate_name,
                addl_plugins,
                |expanded_crate| {
                    let mut state = CompileState::state_after_expand(
                        input,
                        sess,
                        outdir,
                        output,
                        &cstore,
                        expanded_crate,
                        &crate_name,
                    );
                    controller_entry_point!(after_expand, sess, state, Ok(()));
                    Ok(())
                },
            )?
        };

        let output_paths = generated_output_paths(sess, &outputs, output.is_some(), &crate_name);

        // Ensure the source file isn't accidentally overwritten during compilation.
        if let Some(ref input_path) = *input_path {
            if sess.opts.will_create_output_file() {
                if output_contains_path(&output_paths, input_path) {
                    sess.err(&format!(
                        "the input file \"{}\" would be overwritten by the generated \
                         executable",
                        input_path.display()
                    ));
                    return Err(CompileIncomplete::Stopped);
                }
                if let Some(dir_path) = output_conflicts_with_dir(&output_paths) {
                    sess.err(&format!(
                        "the generated executable for the input file \"{}\" conflicts with the \
                         existing directory \"{}\"",
                        input_path.display(),
                        dir_path.display()
                    ));
                    return Err(CompileIncomplete::Stopped);
                }
            }
        }

        write_out_deps(sess, &outputs, &output_paths);
        if sess.opts.output_types.contains_key(&OutputType::DepInfo)
            && sess.opts.output_types.len() == 1
        {
            return Ok(());
        }

        if let &Some(ref dir) = outdir {
            if fs::create_dir_all(dir).is_err() {
                sess.err("failed to find or create the directory specified by --out-dir");
                return Err(CompileIncomplete::Stopped);
            }
        }

        // Construct the HIR map
        let hir_map = time(sess, "indexing hir", || {
            hir_map::map_crate(sess, cstore, &mut hir_forest, &defs)
        });

        {
            hir_map.dep_graph.assert_ignored();
            controller_entry_point!(
                after_hir_lowering,
                sess,
                CompileState::state_after_hir_lowering(
                    input,
                    sess,
                    outdir,
                    output,
                    &cstore,
                    &hir_map,
                    &resolutions,
                    &expanded_crate,
                    &hir_map.krate(),
                    &outputs,
                    &crate_name
                ),
                Ok(())
            );
        }

        let opt_crate = if control.keep_ast {
            Some(&expanded_crate)
        } else {
            drop(expanded_crate);
            None
        };

        let mut arenas = AllArenas::new();

        phase_3_run_analysis_passes(
            &*codegen_backend,
            control,
            sess,
            cstore,
            hir_map,
            resolutions,
            &mut arenas,
            &crate_name,
            &outputs,
            |tcx, rx, result| {
                {
                    // Eventually, we will want to track plugins.
                    tcx.dep_graph.with_ignore(|| {
                        let mut state = CompileState::state_after_analysis(
                            input,
                            sess,
                            outdir,
                            output,
                            opt_crate,
                            tcx.hir().krate(),
                            tcx,
                            &crate_name,
                        );
                        (control.after_analysis.callback)(&mut state);
                    });

                    if control.after_analysis.stop == Compilation::Stop {
                        return result.and_then(|_| Err(CompileIncomplete::Stopped));
                    }
                }

                result?;

                if log_enabled!(::log::Level::Info) {
                    println!("Pre-codegen");
                    tcx.print_debug_stats();
                }

                let ongoing_codegen = phase_4_codegen(&*codegen_backend, tcx, rx);

                if log_enabled!(::log::Level::Info) {
                    println!("Post-codegen");
                    tcx.print_debug_stats();
                }

                if tcx.sess.opts.output_types.contains_key(&OutputType::Mir) {
                    if let Err(e) = mir::transform::dump_mir::emit_mir(tcx, &outputs) {
                        sess.err(&format!("could not emit MIR: {}", e));
                        sess.abort_if_errors();
                    }
                }

                if tcx.sess.opts.debugging_opts.query_stats {
                    tcx.queries.print_stats();
                }

                Ok((outputs.clone(), ongoing_codegen, tcx.dep_graph.clone()))
            },
        )??
    };

    if sess.opts.debugging_opts.print_type_sizes {
        sess.code_stats.borrow().print_type_sizes();
    }

    codegen_backend.join_codegen_and_link(ongoing_codegen, sess, &dep_graph, &outputs)?;

    if sess.opts.debugging_opts.perf_stats {
        sess.print_perf_stats();
    }

    if sess.opts.debugging_opts.self_profile {
        sess.print_profiler_results();
    }

    if sess.opts.debugging_opts.profile_json {
        sess.save_json_results();
    }

    controller_entry_point!(
        compilation_done,
        sess,
        CompileState::state_when_compilation_done(input, sess, outdir, output),
        Ok(())
    );

    Ok(())
}
