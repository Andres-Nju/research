    fn get_orig_src_map(
        &self,
        fm: &SourceFile,
        input_src_map: &InputSourceMap,
        is_default: bool,
    ) -> Result<Option<sourcemap::SourceMap>, Error> {
        self.run(|| -> Result<_, Error> {
            let name = &fm.name;

            let read_inline_sourcemap =
                |data_url: Option<&str>| -> Result<Option<sourcemap::SourceMap>, Error> {
                    match data_url {
                        Some(data_url) => {
                            let url = Url::parse(data_url).with_context(|| {
                                format!("failed to parse inline source map url\n{}", data_url)
                            })?;

                            let idx = match url.path().find("base64,") {
                                Some(v) => v,
                                None => {
                                    bail!(
                                        "failed to parse inline source map: not base64: {:?}",
                                        url
                                    )
                                }
                            };

                            let content = url.path()[idx + "base64,".len()..].trim();

                            let res = base64::decode_config(
                                content.as_bytes(),
                                base64::Config::new(base64::CharacterSet::Standard, true),
                            )
                            .context("failed to decode base64-encoded source map")?;

                            Ok(Some(sourcemap::SourceMap::from_slice(&res).context(
                                "failed to read input source map from inlined base64 encoded \
                                 string",
                            )?))
                        }
                        None => {
                            bail!("failed to parse inline source map: `sourceMappingURL` not found")
                        }
                    }
                };

            let read_sourcemap = || -> Result<Option<sourcemap::SourceMap>, Error> {
                let s = "sourceMappingURL=";
                let idx = fm.src.rfind(s);
                let data_url = idx.map(|idx| &fm.src[idx + s.len()..]);

                match read_inline_sourcemap(data_url) {
                    Ok(r) => Ok(r),
                    Err(_) => {
                        // Load original source map if possible
                        match &name {
                            FileName::Real(filename) => {
                                let dir = match filename.parent() {
                                    Some(v) => v,
                                    None => {
                                        bail!("unexpected: root directory is given as a input file")
                                    }
                                };

                                let map_path = match data_url {
                                    Some(data_url) => {
                                        let mut map_path = dir.join(data_url);
                                        if !map_path.exists() {
                                            // Old behavior. This check would prevent
                                            // regressions.
                                            // Perhaps it shouldn't be supported. Sometimes
                                            // developers don't want to expose their source code.
                                            // Map files are for internal troubleshooting
                                            // convenience.
                                            map_path = PathBuf::from(format!(
                                                "{}.map",
                                                filename.display()
                                            ));
                                            if !map_path.exists() {
                                                bail!("failed to find input source map file")
                                            }
                                        }

                                        Some(map_path)
                                    }
                                    None => {
                                        // Old behavior.
                                        let map_path =
                                            PathBuf::from(format!("{}.map", filename.display()));
                                        if map_path.exists() {
                                            Some(map_path)
                                        } else {
                                            None
                                        }
                                    }
                                };

                                match map_path {
                                    Some(map_path) => {
                                        let path = map_path.display().to_string();
                                        let file = File::open(&path);

                                        // Old behavior.
                                        let file = if !is_default {
                                            file?
                                        } else {
                                            match file {
                                                Ok(v) => v,
                                                Err(_) => return Ok(None),
                                            }
                                        };

                                        Ok(Some(
                                            sourcemap::SourceMap::from_reader(file).with_context(
                                                || {
                                                    format!(
                                                        "failed to read input source map
                                        from file at {}",
                                                        path
                                                    )
                                                },
                                            )?,
                                        ))
                                    }
                                    None => Ok(None),
                                }
                            }
                            _ => {
                                tracing::error!("Failed to load source map for non-file input");
                                Ok(None)
                            }
                        }
                    }
                }
            };

            // Load original source map
            match input_src_map {
                InputSourceMap::Bool(false) => Ok(None),
                InputSourceMap::Bool(true) => read_sourcemap(),
                InputSourceMap::Str(ref s) => {
                    if s == "inline" {
                        read_sourcemap()
                    } else {
                        // Load source map passed by user
                        Ok(Some(
                            sourcemap::SourceMap::from_slice(s.as_bytes()).context(
                                "failed to read input source map from user-provided sourcemap",
                            )?,
                        ))
                    }
                }
            }
        })
    }
