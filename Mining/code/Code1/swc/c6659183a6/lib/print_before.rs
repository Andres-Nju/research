    pub fn print<T>(
        &self,
        node: &T,
        source_file_name: Option<&str>,
        output_path: Option<PathBuf>,
        inline_sources_content: bool,
        target: EsVersion,
        source_map: SourceMapsConfig,
        source_map_names: &AHashMap<BytePos, JsWord>,
        orig: Option<&sourcemap::SourceMap>,
        minify: bool,
        comments: Option<&dyn Comments>,
        emit_source_map_columns: bool,
        ascii_only: bool,
    ) -> Result<TransformOutput, Error>
    where
        T: Node + VisitWith<IdentCollector>,
    {
        self.run(|| {
            let _timer = timer!("Compiler.print");

            let mut src_map_buf = vec![];

            let src = {
                let mut buf = vec![];
                {
                    let mut wr = Box::new(swc_ecma_codegen::text_writer::JsWriter::new(
                        self.cm.clone(),
                        "\n",
                        &mut buf,
                        if source_map.enabled() {
                            Some(&mut src_map_buf)
                        } else {
                            None
                        },
                    )) as Box<dyn WriteJs>;

                    if minify {
                        wr = Box::new(swc_ecma_codegen::text_writer::omit_trailing_semi(wr));
                    }

                    let mut emitter = Emitter {
                        cfg: swc_ecma_codegen::Config {
                            minify,
                            target,
                            ascii_only,
                            ..Default::default()
                        },
                        comments,
                        cm: self.cm.clone(),
                        wr,
                    };

                    node.emit_with(&mut emitter)
                        .context("failed to emit module")?;
                }
                // Invalid utf8 is valid in javascript world.
                String::from_utf8(buf).expect("invalid utf8 character detected")
            };

            if cfg!(debug_assertions)
                && !src_map_buf.is_empty()
                && src_map_buf.iter().all(|(bp, _)| bp.is_dummy())
                && src.lines().count() >= 3
                && option_env!("SWC_DEBUG") == Some("1")
            {
                panic!("The module contains only dummy spans\n{}", src);
            }

            let (code, map) = match source_map {
                SourceMapsConfig::Bool(v) => {
                    if v {
                        let mut buf = vec![];

                        self.cm
                            .build_source_map_with_config(
                                &src_map_buf,
                                orig,
                                SwcSourceMapConfig {
                                    source_file_name,
                                    output_path: output_path.as_deref(),
                                    names: source_map_names,
                                    inline_sources_content,
                                    emit_columns: emit_source_map_columns,
                                },
                            )
                            .to_writer(&mut buf)
                            .context("failed to write source map")?;
                        let map = String::from_utf8(buf).context("source map is not utf-8")?;
                        (src, Some(map))
                    } else {
                        (src, None)
                    }
                }
                SourceMapsConfig::Str(_) => {
                    let mut src = src;

                    let mut buf = vec![];

                    self.cm
                        .build_source_map_with_config(
                            &src_map_buf,
                            orig,
                            SwcSourceMapConfig {
                                source_file_name,
                                output_path: output_path.as_deref(),
                                names: source_map_names,
                                inline_sources_content,
                                emit_columns: emit_source_map_columns,
                            },
                        )
                        .to_writer(&mut buf)
                        .context("failed to write source map file")?;
                    let map = String::from_utf8(buf).context("source map is not utf-8")?;

                    src.push_str("\n//# sourceMappingURL=data:application/json;base64,");
                    base64::encode_config_buf(
                        map.as_bytes(),
                        base64::Config::new(base64::CharacterSet::Standard, true),
                        &mut src,
                    );
                    (src, None)
                }
            };

            Ok(TransformOutput { code, map })
        })
    }
