    pub fn file_types(
        &self,
        crate_type: &str,
        flavor: FileFlavor,
        kind: &TargetKind,
        target_triple: &str,
    ) -> CargoResult<Option<Vec<FileType>>> {
        let mut crate_types = self.crate_types.borrow_mut();
        let entry = crate_types.entry(crate_type.to_string());
        let crate_type_info = match entry {
            Entry::Occupied(o) => &*o.into_mut(),
            Entry::Vacant(v) => {
                let value = self.discover_crate_type(v.key())?;
                &*v.insert(value)
            }
        };
        let (prefix, suffix) = match *crate_type_info {
            Some((ref prefix, ref suffix)) => (prefix, suffix),
            None => return Ok(None),
        };
        let mut ret = vec![FileType {
            suffix: suffix.clone(),
            prefix: prefix.clone(),
            flavor,
            should_replace_hyphens: false,
        }];

        // See rust-lang/cargo#4500.
        if target_triple.ends_with("pc-windows-msvc")
            && crate_type.ends_with("dylib")
            && suffix == ".dll"
        {
            ret.push(FileType {
                suffix: ".dll.lib".to_string(),
                prefix: prefix.clone(),
                flavor: FileFlavor::Normal,
                should_replace_hyphens: false,
            })
        }

        // See rust-lang/cargo#4535.
        if target_triple.starts_with("wasm32-") && crate_type == "bin" && suffix == ".js" {
            ret.push(FileType {
                suffix: ".wasm".to_string(),
                prefix: prefix.clone(),
                flavor: FileFlavor::Auxiliary,
                should_replace_hyphens: true,
            })
        }

        // See rust-lang/cargo#4490, rust-lang/cargo#4960.
        // Only uplift debuginfo for binaries.
        // - Tests are run directly from `target/debug/deps/` with the
        //   metadata hash still in the filename.
        // - Examples are only uplifted for apple because the symbol file
        //   needs to match the executable file name to be found (i.e., it
        //   needs to remove the hash in the filename). On Windows, the path
        //   to the .pdb with the hash is embedded in the executable.
        let is_apple = target_triple.contains("-apple-");
        if *kind == TargetKind::Bin || (*kind == TargetKind::ExampleBin && is_apple) {
            if is_apple {
                ret.push(FileType {
                    suffix: ".dSYM".to_string(),
                    prefix: prefix.clone(),
                    flavor: FileFlavor::DebugInfo,
                    should_replace_hyphens: false,
                })
            } else if target_triple.ends_with("-msvc") {
                ret.push(FileType {
                    suffix: ".pdb".to_string(),
                    prefix: prefix.clone(),
                    flavor: FileFlavor::DebugInfo,
                    should_replace_hyphens: false,
                })
            }
        }

        Ok(Some(ret))
    }
