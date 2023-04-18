    fn check_filesystem(
        &mut self,
        pkg_root: &Path,
        target_root: &Path,
        mtime_on_use: bool,
    ) -> CargoResult<()> {
        assert!(!self.fs_status.up_to_date());

        let mut mtimes = HashMap::new();

        // Get the `mtime` of all outputs. Optionally update their mtime
        // afterwards based on the `mtime_on_use` flag. Afterwards we want the
        // minimum mtime as it's the one we'll be comparing to inputs and
        // dependencies.
        for output in self.outputs.iter() {
            let mtime = match paths::mtime(output) {
                Ok(mtime) => mtime,

                // This path failed to report its `mtime`. It probably doesn't
                // exists, so leave ourselves as stale and bail out.
                Err(e) => {
                    log::debug!("failed to get mtime of {:?}: {}", output, e);
                    return Ok(());
                }
            };
            if mtime_on_use {
                let t = FileTime::from_system_time(SystemTime::now());
                filetime::set_file_times(output, t, t)?;
            }
            assert!(mtimes.insert(output.clone(), mtime).is_none());
        }

        let max_mtime = match mtimes.values().max() {
            Some(mtime) => mtime,

            // We had no output files. This means we're an overridden build
            // script and we're just always up to date because we aren't
            // watching the filesystem.
            None => {
                self.fs_status = FsStatus::UpToDate { mtimes };
                return Ok(());
            }
        };

        for dep in self.deps.iter() {
            let dep_mtimes = match &dep.fingerprint.fs_status {
                FsStatus::UpToDate { mtimes } => mtimes,
                // If our dependency is stale, so are we, so bail out.
                FsStatus::Stale => return Ok(()),
            };

            // If our dependency edge only requires the rmeta file to be present
            // then we only need to look at that one output file, otherwise we
            // need to consider all output files to see if we're out of date.
            let dep_mtime = if dep.only_requires_rmeta {
                dep_mtimes
                    .iter()
                    .filter_map(|(path, mtime)| {
                        if path.extension().and_then(|s| s.to_str()) == Some("rmeta") {
                            Some(mtime)
                        } else {
                            None
                        }
                    })
                    .next()
                    .expect("failed to find rmeta")
            } else {
                match dep_mtimes.values().max() {
                    Some(mtime) => mtime,
                    // If our dependencies is up to date and has no filesystem
                    // interactions, then we can move on to the next dependency.
                    None => continue,
                }
            };

            // If the dependency is newer than our own output then it was
            // recompiled previously. We transitively become stale ourselves in
            // that case, so bail out.
            //
            // Note that this comparison should probably be `>=`, not `>`, but
            // for a discussion of why it's `>` see the discussion about #5918
            // below in `find_stale`.
            if dep_mtime > max_mtime {
                log::info!("dependency on `{}` is newer than we are", dep.name);
                return Ok(());
            }
        }

        // If we reached this far then all dependencies are up to date. Check
        // all our `LocalFingerprint` information to see if we have any stale
        // files for this package itself. If we do find something log a helpful
        // message and bail out so we stay stale.
        for local in self.local.get_mut().unwrap().iter() {
            if let Some(file) = local.find_stale_file(pkg_root, target_root)? {
                file.log();
                return Ok(());
            }
        }

        // Everything was up to date! Record such.
        self.fs_status = FsStatus::UpToDate { mtimes };

        Ok(())
    }
