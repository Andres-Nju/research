fn rustc(cx: &mut Context, unit: &Unit) -> CargoResult<Work> {
    let crate_types = unit.target.rustc_crate_types();
    let mut rustc = try!(prepare_rustc(cx, crate_types, unit));

    let name = unit.pkg.name().to_string();
    if !cx.show_warnings(unit.pkg.package_id()) {
        if try!(cx.config.rustc()).cap_lints {
            rustc.arg("--cap-lints").arg("allow");
        } else {
            rustc.arg("-Awarnings");
        }
    }
    let has_custom_args = unit.profile.rustc_args.is_some();
    let exec_engine = cx.exec_engine.clone();

    let filenames = try!(cx.target_filenames(unit));
    let root = cx.out_dir(unit);

    // Prepare the native lib state (extra -L and -l flags)
    let build_state = cx.build_state.clone();
    let current_id = unit.pkg.package_id().clone();
    let build_deps = load_build_deps(cx, unit);

    // If we are a binary and the package also contains a library, then we
    // don't pass the `-l` flags.
    let pass_l_flag = unit.target.is_lib() ||
                      !unit.pkg.targets().iter().any(|t| t.is_lib());
    let do_rename = unit.target.allows_underscores() && !unit.profile.test;
    let real_name = unit.target.name().to_string();
    let crate_name = unit.target.crate_name();
    let move_outputs_up = unit.pkg.package_id() == cx.resolve.root();

    let rustc_dep_info_loc = if do_rename {
        root.join(&crate_name)
    } else {
        root.join(&cx.file_stem(unit))
    }.with_extension("d");
    let dep_info_loc = fingerprint::dep_info_loc(cx, unit);
    let cwd = cx.config.cwd().to_path_buf();

    rustc.args(&try!(cx.rustflags_args(unit)));

    return Ok(Work::new(move |state| {
        // Only at runtime have we discovered what the extra -L and -l
        // arguments are for native libraries, so we process those here. We
        // also need to be sure to add any -L paths for our plugins to the
        // dynamic library load path as a plugin's dynamic library may be
        // located somewhere in there.
        if let Some(build_deps) = build_deps {
            let build_state = build_state.outputs.lock().unwrap();
            try!(add_native_deps(&mut rustc, &build_state, &build_deps,
                                 pass_l_flag, &current_id));
            try!(add_plugin_deps(&mut rustc, &build_state, &build_deps));
        }

        // FIXME(rust-lang/rust#18913): we probably shouldn't have to do
        //                              this manually
        for &(ref filename, _linkable) in filenames.iter() {
            let dst = root.join(filename);
            if fs::metadata(&dst).is_ok() {
                try!(fs::remove_file(&dst));
            }
        }

        state.running(&rustc);
        try!(exec_engine.exec(rustc).chain_error(|| {
            human(format!("Could not compile `{}`.", name))
        }));

        if do_rename && real_name != crate_name {
            let dst = root.join(&filenames[0].0);
            let src = dst.with_file_name(dst.file_name().unwrap()
                                            .to_str().unwrap()
                                            .replace(&real_name, &crate_name));
            if !has_custom_args || src.exists() {
                try!(fs::rename(&src, &dst).chain_error(|| {
                    internal(format!("could not rename crate {:?}", src))
                }));
            }
        }

        if !has_custom_args || fs::metadata(&rustc_dep_info_loc).is_ok() {
            try!(fs::rename(&rustc_dep_info_loc, &dep_info_loc).chain_error(|| {
                internal(format!("could not rename dep info: {:?}",
                              rustc_dep_info_loc))
            }));
            try!(fingerprint::append_current_dir(&dep_info_loc, &cwd));
        }

        // If we're a "root crate", e.g. the target of this compilation, then we
        // hard link our outputs out of the `deps` directory into the directory
        // above. This means that `cargo build` will produce binaries in
        // `target/debug` which one probably expects.
        if move_outputs_up {
            for &(ref filename, _linkable) in filenames.iter() {
                let src = root.join(filename);
                // This may have been a `cargo rustc` command which changes the
                // output, so the source may not actually exist.
                if !src.exists() {
                    continue
                }

                // We currently only lift files up from the `deps` directory. If
                // it was compiled into something like `example/` or `doc/` then
                // we don't want to link it up.
                let src_dir = src.parent().unwrap();
                if !src_dir.ends_with("deps") {
                    continue
                }
                let dst = src_dir.parent().unwrap()
                                 .join(src.file_name().unwrap());
                if dst.exists() {
                    try!(fs::remove_file(&dst).chain_error(|| {
                        human(format!("failed to remove: {}", dst.display()))
                    }));
                }
                try!(fs::hard_link(&src, &dst).chain_error(|| {
                    human(format!("failed to link `{}` to `{}`",
                                  src.display(), dst.display()))
                }));
            }
        }

        Ok(())
    }));

    // Add all relevant -L and -l flags from dependencies (now calculated and
    // present in `state`) to the command provided
    fn add_native_deps(rustc: &mut CommandPrototype,
                       build_state: &BuildMap,
                       build_scripts: &BuildScripts,
                       pass_l_flag: bool,
                       current_id: &PackageId) -> CargoResult<()> {
        for key in build_scripts.to_link.iter() {
            let output = try!(build_state.get(key).chain_error(|| {
                internal(format!("couldn't find build state for {}/{:?}",
                                 key.0, key.1))
            }));
            for path in output.library_paths.iter() {
                rustc.arg("-L").arg(path);
            }
            if key.0 == *current_id {
                for cfg in &output.cfgs {
                    rustc.arg("--cfg").arg(cfg);
                }
                if pass_l_flag {
                    for name in output.library_links.iter() {
                        rustc.arg("-l").arg(name);
                    }
                }
            }
        }
        Ok(())
    }
}
