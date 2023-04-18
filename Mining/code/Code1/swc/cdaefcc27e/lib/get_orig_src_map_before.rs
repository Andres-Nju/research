    fn get_orig_src_map(
        &self,
        fm: &SourceFile,
        input_src_map: &InputSourceMap,
    ) -> Result<Option<sourcemap::SourceMap>, Error> {
        self.run(|| -> Result<_, Error> {
            let name = &fm.name;

            // Load original source map
            match input_src_map {
                InputSourceMap::Bool(false) => Ok(None),
                InputSourceMap::Bool(true) => {
                    // Load original source map if possible
                    match &name {
                        FileName::Real(filename) => {
                            let path = format!("{}.map", filename.display());
                            let file = File::open(&path)
                                .context("failed to open input source map file")?;
                            Ok(Some(sourcemap::SourceMap::from_reader(file).with_context(
                                || format!("failed to read input source map from file at {}", path),
                            )?))
                        }
                        _ => {
                            log::error!("Failed to load source map for non-file input");
                            return Ok(None);
                        }
                    }
                }
                InputSourceMap::Str(ref s) => {
                    if s == "inline" {
                        // Load inline source map by simple string
                        // operations
                        let s = "sourceMappingURL=data:application/json;base64,";
                        let idx = fm.src.rfind(s);
                        let idx = match idx {
                            None => bail!(
                                "failed to parse inline source map: `sourceMappingURL` not found"
                            ),
                            Some(v) => v,
                        };
                        let encoded = &s[idx + s.len()..];

                        let res = base64::decode(encoded.as_bytes())
                            .context("failed to decode base64-encoded source map")?;

                        Ok(Some(sourcemap::SourceMap::from_slice(&res).context(
                            "failed to read input source map from inlined base64 encoded string",
                        )?))
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
