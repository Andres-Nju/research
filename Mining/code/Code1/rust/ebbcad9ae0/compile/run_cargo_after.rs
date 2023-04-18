fn run_cargo(build: &Build, cargo: &mut Command, stamp: &Path) {
    // Instruct Cargo to give us json messages on stdout, critically leaving
    // stderr as piped so we can get those pretty colors.
    cargo.arg("--message-format").arg("json")
         .stdout(Stdio::piped());

    if stderr_isatty() && build.ci_env == CiEnv::None {
        // since we pass message-format=json to cargo, we need to tell the rustc
        // wrapper to give us colored output if necessary. This is because we
        // only want Cargo's JSON output, not rustcs.
        cargo.env("RUSTC_COLOR", "1");
    }

    build.verbose(&format!("running: {:?}", cargo));
    let mut child = match cargo.spawn() {
        Ok(child) => child,
        Err(e) => panic!("failed to execute command: {:?}\nerror: {}", cargo, e),
    };

    // `target_root_dir` looks like $dir/$target/release
    let target_root_dir = stamp.parent().unwrap();
    // `target_deps_dir` looks like $dir/$target/release/deps
    let target_deps_dir = target_root_dir.join("deps");
    // `host_root_dir` looks like $dir/release
    let host_root_dir = target_root_dir.parent().unwrap() // chop off `release`
                                       .parent().unwrap() // chop off `$target`
                                       .join(target_root_dir.file_name().unwrap());

    // Spawn Cargo slurping up its JSON output. We'll start building up the
    // `deps` array of all files it generated along with a `toplevel` array of
    // files we need to probe for later.
    let mut deps = Vec::new();
    let mut toplevel = Vec::new();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    for line in stdout.lines() {
        let line = t!(line);
        let json: serde_json::Value = if line.starts_with("{") {
            t!(serde_json::from_str(&line))
        } else {
            // If this was informational, just print it out and continue
            println!("{}", line);
            continue
        };
        if json["reason"].as_str() != Some("compiler-artifact") {
            continue
        }
        for filename in json["filenames"].as_array().unwrap() {
            let filename = filename.as_str().unwrap();
            // Skip files like executables
            if !filename.ends_with(".rlib") &&
               !filename.ends_with(".lib") &&
               !is_dylib(&filename) {
                continue
            }

            let filename = Path::new(filename);

            // If this was an output file in the "host dir" we don't actually
            // worry about it, it's not relevant for us.
            if filename.starts_with(&host_root_dir) {
                continue;
            }

            // If this was output in the `deps` dir then this is a precise file
            // name (hash included) so we start tracking it.
            if filename.starts_with(&target_deps_dir) {
                deps.push(filename.to_path_buf());
                continue;
            }

            // Otherwise this was a "top level artifact" which right now doesn't
            // have a hash in the name, but there's a version of this file in
            // the `deps` folder which *does* have a hash in the name. That's
            // the one we'll want to we'll probe for it later.
            toplevel.push((filename.file_stem().unwrap()
                                    .to_str().unwrap().to_string(),
                            filename.extension().unwrap().to_owned()
                                    .to_str().unwrap().to_string()));
        }
    }

    // Make sure Cargo actually succeeded after we read all of its stdout.
    let status = t!(child.wait());
    if !status.success() {
        panic!("command did not execute successfully: {:?}\n\
                expected success, got: {}",
               cargo,
               status);
    }

    // Ok now we need to actually find all the files listed in `toplevel`. We've
    // got a list of prefix/extensions and we basically just need to find the
    // most recent file in the `deps` folder corresponding to each one.
    let contents = t!(target_deps_dir.read_dir())
        .map(|e| t!(e))
        .map(|e| (e.path(), e.file_name().into_string().unwrap(), t!(e.metadata())))
        .collect::<Vec<_>>();
    for (prefix, extension) in toplevel {
        let candidates = contents.iter().filter(|&&(_, ref filename, _)| {
            filename.starts_with(&prefix[..]) &&
                filename[prefix.len()..].starts_with("-") &&
                filename.ends_with(&extension[..])
        });
        let max = candidates.max_by_key(|&&(_, _, ref metadata)| {
            FileTime::from_last_modification_time(metadata)
        });
        let path_to_add = match max {
            Some(triple) => triple.0.to_str().unwrap(),
            None => panic!("no output generated for {:?} {:?}", prefix, extension),
        };
        if is_dylib(path_to_add) {
            let candidate = format!("{}.lib", path_to_add);
            let candidate = PathBuf::from(candidate);
            if candidate.exists() {
                deps.push(candidate);
            }
        }
        deps.push(path_to_add.into());
    }

    // Now we want to update the contents of the stamp file, if necessary. First
    // we read off the previous contents along with its mtime. If our new
    // contents (the list of files to copy) is different or if any dep's mtime
    // is newer then we rewrite the stamp file.
    deps.sort();
    let mut stamp_contents = Vec::new();
    if let Ok(mut f) = File::open(stamp) {
        t!(f.read_to_end(&mut stamp_contents));
    }
    let stamp_mtime = mtime(&stamp);
    let mut new_contents = Vec::new();
    let mut max = None;
    let mut max_path = None;
    for dep in deps {
        let mtime = mtime(&dep);
        if Some(mtime) > max {
            max = Some(mtime);
            max_path = Some(dep.clone());
        }
        new_contents.extend(dep.to_str().unwrap().as_bytes());
        new_contents.extend(b"\0");
    }
    let max = max.unwrap();
    let max_path = max_path.unwrap();
    if stamp_contents == new_contents && max <= stamp_mtime {
        build.verbose(&format!("not updating {:?}; contents equal and {} <= {}",
                stamp, max, stamp_mtime));
        return
    }
    if max > stamp_mtime {
        build.verbose(&format!("updating {:?} as {:?} changed", stamp, max_path));
    } else {
        build.verbose(&format!("updating {:?} as deps changed", stamp));
    }
    t!(t!(File::create(stamp)).write_all(&new_contents));
}
