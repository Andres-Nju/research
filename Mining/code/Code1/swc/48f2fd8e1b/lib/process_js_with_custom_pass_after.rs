    pub fn process_js_with_custom_pass<P1, P2>(
        &self,
        fm: Arc<SourceFile>,
        program: Option<Program>,
        handler: &Handler,
        opts: &Options,
        custom_before_pass: impl FnOnce(&Program, &SingleThreadedComments) -> P1,
        custom_after_pass: impl FnOnce(&Program, &SingleThreadedComments) -> P2,
    ) -> Result<TransformOutput, Error>
    where
        P1: swc_ecma_visit::Fold,
        P2: swc_ecma_visit::Fold,
    {
        self.run(|| -> Result<_, Error> {
            let comments = SingleThreadedComments::default();
            let config = self.run(|| {
                self.parse_js_as_input(
                    fm.clone(),
                    program,
                    handler,
                    opts,
                    &fm.name,
                    Some(&comments),
                    |program| custom_before_pass(program, &comments),
                )
            })?;
            let config = match config {
                Some(v) => v,
                None => {
                    bail!("cannot process file because it's ignored by .swcrc")
                }
            };

            let pass = chain!(config.pass, custom_after_pass(&config.program, &comments));

            let config = BuiltInput {
                program: config.program,
                pass,
                syntax: config.syntax,
                target: config.target,
                minify: config.minify,
                external_helpers: config.external_helpers,
                source_maps: config.source_maps,
                input_source_map: config.input_source_map,
                is_module: config.is_module,
                output_path: config.output_path,
                source_file_name: config.source_file_name,
                preserve_comments: config.preserve_comments,
                inline_sources_content: config.inline_sources_content,
                comments: config.comments,
            };

            let orig = if config.source_maps.enabled() {
                self.get_orig_src_map(&fm, &config.input_source_map, false)?
            } else {
                None
            };

            self.process_js_inner(handler, orig.as_ref(), config)
        })
        .context("failed to process input file")
    }
