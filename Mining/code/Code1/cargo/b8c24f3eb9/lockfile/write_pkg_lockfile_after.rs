pub fn write_pkg_lockfile(ws: &Workspace, resolve: &Resolve) -> CargoResult<()> {
    // Load the original lockfile if it exists.
    let ws_root = Filesystem::new(ws.root().to_path_buf());
    let orig = ws_root.open_ro("Cargo.lock", ws.config(), "Cargo.lock file");
    let orig = orig.and_then(|mut f| {
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(s)
    });

    let toml = toml::Value::try_from(WorkspaceResolve { ws, resolve }).unwrap();

    let mut out = String::new();

    let deps = toml["package"].as_array().unwrap();
    for dep in deps.iter() {
        let dep = dep.as_table().unwrap();

        out.push_str("[[package]]\n");
        emit_package(dep, &mut out);
    }

    if let Some(patch) = toml.get("patch") {
        let list = patch["unused"].as_array().unwrap();
        for entry in list {
            out.push_str("[[patch.unused]]\n");
            emit_package(entry.as_table().unwrap(), &mut out);
            out.push_str("\n");
        }
    }

    if let Some(meta) = toml.get("metadata") {
        out.push_str("[metadata]\n");
        out.push_str(&meta.to_string());
    }

    // If the lockfile contents haven't changed so don't rewrite it. This is
    // helpful on read-only filesystems.
    if let Ok(orig) = orig {
        if are_equal_lockfiles(orig, &out, ws) {
            return Ok(());
        }
    }

    if !ws.config().lock_update_allowed() {
        if ws.config().cli_unstable().offline {
            bail!("can't update in the offline mode");
        }

        let flag = if ws.config().network_allowed() {
            "--locked"
        } else {
            "--frozen"
        };
        bail!(
            "the lock file {} needs to be updated but {} was passed to \
             prevent this",
            ws.root().to_path_buf().join("Cargo.lock").display(),
            flag
        );
    }

    // Ok, if that didn't work just write it out
    ws_root
        .open_rw("Cargo.lock", ws.config(), "Cargo.lock file")
        .and_then(|mut f| {
            f.file().set_len(0)?;
            f.write_all(out.as_bytes())?;
            Ok(())
        })
        .chain_err(|| format!("failed to write {}", ws.root().join("Cargo.lock").display()))?;
    Ok(())
}
