pub fn prepare<'a, 'cfg>(cx: &mut Context<'a, 'cfg>, unit: &Unit<'a>)
                         -> CargoResult<(Work, Work, Freshness)> {
    let _p = profile::start(format!("build script prepare: {}/{}",
                                    unit.pkg, unit.target.name()));
    let overridden = cx.build_state.has_override(unit);
    let (work_dirty, work_fresh) = if overridden {
        (Work::new(|_| Ok(())), Work::new(|_| Ok(())))
    } else {
        build_work(cx, unit)?
    };

    // Now that we've prep'd our work, build the work needed to manage the
    // fingerprint and then start returning that upwards.
    let (freshness, dirty, fresh) =
            fingerprint::prepare_build_cmd(cx, unit)?;

    Ok((work_dirty.then(dirty), work_fresh.then(fresh), freshness))
}

fn build_work<'a, 'cfg>(cx: &mut Context<'a, 'cfg>, unit: &Unit<'a>)
                        -> CargoResult<(Work, Work)> {
    let dependencies = cx.dep_run_custom_build(unit)?;
    let build_script_unit = dependencies.iter().find(|d| {
        !d.profile.run_custom_build && d.target.is_custom_build()
    }).expect("running a script not depending on an actual script");
    let script_output = cx.build_script_dir(build_script_unit);
    let build_output = cx.build_script_out_dir(unit);

    // Building the command to execute
    let to_exec = script_output.join(unit.target.name());

    // Start preparing the process to execute, starting out with some
    // environment variables. Note that the profile-related environment
    // variables are not set with this the build script's profile but rather the
    // package's library profile.
    let profile = cx.lib_profile();
    let to_exec = to_exec.into_os_string();
    let mut cmd = cx.compilation.host_process(to_exec, unit.pkg)?;
    cmd.env("OUT_DIR", &build_output)
       .env("CARGO_MANIFEST_DIR", unit.pkg.root())
       .env("NUM_JOBS", &cx.jobs().to_string())
       .env("TARGET", &match unit.kind {
           Kind::Host => cx.host_triple(),
           Kind::Target => cx.target_triple(),
       })
       .env("DEBUG", &profile.debuginfo.is_some().to_string())
       .env("OPT_LEVEL", &profile.opt_level)
       .env("PROFILE", if cx.build_config.release { "release" } else { "debug" })
       .env("HOST", cx.host_triple())
       .env("RUSTC", &cx.config.rustc()?.path)
       .env("RUSTDOC", &*cx.config.rustdoc()?);

    if let Some(links) = unit.pkg.manifest().links() {
        cmd.env("CARGO_MANIFEST_LINKS", links);
    }

    // Be sure to pass along all enabled features for this package, this is the
    // last piece of statically known information that we have.
    if let Some(features) = cx.resolve.features(unit.pkg.package_id()) {
        for feat in features.iter() {
            cmd.env(&format!("CARGO_FEATURE_{}", super::envify(feat)), "1");
        }
    }

    let mut cfg_map = HashMap::new();
    for cfg in cx.cfg(unit.kind) {
        match *cfg {
            Cfg::Name(ref n) => { cfg_map.insert(n.clone(), None); }
            Cfg::KeyPair(ref k, ref v) => {
                match *cfg_map.entry(k.clone()).or_insert(Some(Vec::new())) {
                    Some(ref mut values) => values.push(v.clone()),
                    None => { /* ... */ }
                }
            }
        }
    }
    for (k, v) in cfg_map {
        let k = format!("CARGO_CFG_{}", super::envify(&k));
        match v {
            Some(list) => { cmd.env(&k, list.join(",")); }
            None => { cmd.env(&k, ""); }
        }
    }

    // Gather the set of native dependencies that this package has along with
    // some other variables to close over.
    //
    // This information will be used at build-time later on to figure out which
    // sorts of variables need to be discovered at that time.
    let lib_deps = {
        dependencies.iter().filter_map(|unit| {
            if unit.profile.run_custom_build {
                Some((unit.pkg.manifest().links().unwrap().to_string(),
                      unit.pkg.package_id().clone()))
            } else {
                None
            }
        }).collect::<Vec<_>>()
    };
    let pkg_name = unit.pkg.to_string();
    let build_state = cx.build_state.clone();
    let id = unit.pkg.package_id().clone();
    let output_file = build_output.parent().unwrap().join("output");
    let all = (id.clone(), pkg_name.clone(), build_state.clone(),
               output_file.clone());
    let build_scripts = super::load_build_deps(cx, unit);
    let kind = unit.kind;
    let json_messages = cx.build_config.json_messages;

    // Check to see if the build script as already run, and if it has keep
    // track of whether it has told us about some explicit dependencies
    let prev_output = BuildOutput::parse_file(&output_file, &pkg_name).ok();
    let rerun_if_changed = match prev_output {
        Some(ref prev) => prev.rerun_if_changed.clone(),
        None => Vec::new(),
    };
    cx.build_explicit_deps.insert(*unit, (output_file.clone(), rerun_if_changed));

    fs::create_dir_all(&script_output)?;
    fs::create_dir_all(&build_output)?;

    // Prepare the unit of "dirty work" which will actually run the custom build
    // command.
    //
    // Note that this has to do some extra work just before running the command
    // to determine extra environment variables and such.
    let dirty = Work::new(move |state| {
        // Make sure that OUT_DIR exists.
        //
        // If we have an old build directory, then just move it into place,
        // otherwise create it!
        if fs::metadata(&build_output).is_err() {
            fs::create_dir(&build_output).chain_error(|| {
                internal("failed to create script output directory for \
                          build command")
            })?;
        }

        // For all our native lib dependencies, pick up their metadata to pass
        // along to this custom build command. We're also careful to augment our
        // dynamic library search path in case the build script depended on any
        // native dynamic libraries.
        {
            let build_state = build_state.outputs.lock().unwrap();
            for (name, id) in lib_deps {
                let key = (id.clone(), kind);
                let state = build_state.get(&key).chain_error(|| {
                    internal(format!("failed to locate build state for env \
                                      vars: {}/{:?}", id, kind))
                })?;
                let data = &state.metadata;
                for &(ref key, ref value) in data.iter() {
                    cmd.env(&format!("DEP_{}_{}", super::envify(&name),
                                     super::envify(key)), value);
                }
            }
            if let Some(build_scripts) = build_scripts {
                super::add_plugin_deps(&mut cmd, &build_state,
                                            &build_scripts)?;
            }
        }

        // And now finally, run the build command itself!
        state.running(&cmd);
        let output = cmd.exec_with_streaming(
            &mut |out_line| { state.stdout(out_line); Ok(()) },
            &mut |err_line| { state.stderr(err_line); Ok(()) },
        ).map_err(|mut e| {
            e.desc = format!("failed to run custom build command for `{}`\n{}",
                             pkg_name, e.desc);
            Human(e)
        })?;
        paths::write(&output_file, &output.stdout)?;

        // After the build command has finished running, we need to be sure to
        // remember all of its output so we can later discover precisely what it
        // was, even if we don't run the build command again (due to freshness).
        //
        // This is also the location where we provide feedback into the build
        // state informing what variables were discovered via our script as
        // well.
        let parsed_output = BuildOutput::parse(&output.stdout, &pkg_name)?;

        if json_messages {
            let library_paths = parsed_output.library_paths.iter().map(|l| {
                l.display().to_string()
            }).collect::<Vec<_>>();
            machine_message::emit(machine_message::BuildScript {
                package_id: &id,
                linked_libs: &parsed_output.library_links,
                linked_paths: &library_paths,
                cfgs: &parsed_output.cfgs,
            });
        }

        build_state.insert(id, kind, parsed_output);
        Ok(())
    });

    // Now that we've prepared our work-to-do, we need to prepare the fresh work
    // itself to run when we actually end up just discarding what we calculated
    // above.
    let fresh = Work::new(move |_tx| {
        let (id, pkg_name, build_state, output_file) = all;
        let output = match prev_output {
            Some(output) => output,
            None => BuildOutput::parse_file(&output_file, &pkg_name)?,
        };
        build_state.insert(id, kind, output);
        Ok(())
    });

    Ok((dirty, fresh))
}
