    pub(crate) fn smart_resolve_report_errors(
        &mut self,
        path: &[Segment],
        span: Span,
        source: PathSource,
        def: Option<Def>,
    ) -> (DiagnosticBuilder<'a>, Vec<ImportSuggestion>) {
        let ident_span = path.last().map_or(span, |ident| ident.ident.span);
        let ns = source.namespace();
        let is_expected = &|def| source.is_expected(def);
        let is_enum_variant = &|def| if let Def::Variant(..) = def { true } else { false };

        // Make the base error.
        let expected = source.descr_expected();
        let path_str = Segment::names_to_string(path);
        let item_str = path.last().unwrap().ident;
        let code = source.error_code(def.is_some());
        let (base_msg, fallback_label, base_span) = if let Some(def) = def {
            (format!("expected {}, found {} `{}`", expected, def.kind_name(), path_str),
                format!("not a {}", expected),
                span)
        } else {
            let item_span = path.last().unwrap().ident.span;
            let (mod_prefix, mod_str) = if path.len() == 1 {
                (String::new(), "this scope".to_string())
            } else if path.len() == 2 && path[0].ident.name == keywords::PathRoot.name() {
                (String::new(), "the crate root".to_string())
            } else {
                let mod_path = &path[..path.len() - 1];
                let mod_prefix = match self.resolve_path_without_parent_scope(
                    mod_path, Some(TypeNS), false, span, CrateLint::No
                ) {
                    PathResult::Module(ModuleOrUniformRoot::Module(module)) =>
                        module.def(),
                    _ => None,
                }.map_or(String::new(), |def| format!("{} ", def.kind_name()));
                (mod_prefix, format!("`{}`", Segment::names_to_string(mod_path)))
            };
            (format!("cannot find {} `{}` in {}{}", expected, item_str, mod_prefix, mod_str),
                format!("not found in {}", mod_str),
                item_span)
        };

        let code = DiagnosticId::Error(code.into());
        let mut err = self.session.struct_span_err_with_code(base_span, &base_msg, code);

        // Emit help message for fake-self from other languages (e.g., `this` in Javascript).
        if ["this", "my"].contains(&&*item_str.as_str())
            && self.self_value_is_available(path[0].ident.span, span) {
            err.span_suggestion(
                span,
                "did you mean",
                "self".to_string(),
                Applicability::MaybeIncorrect,
            );
        }

        // Emit special messages for unresolved `Self` and `self`.
        if is_self_type(path, ns) {
            __diagnostic_used!(E0411);
            err.code(DiagnosticId::Error("E0411".into()));
            err.span_label(span, format!("`Self` is only available in impls, traits, \
                                          and type definitions"));
            return (err, Vec::new());
        }
        if is_self_value(path, ns) {
            debug!("smart_resolve_path_fragment: E0424, source={:?}", source);

            __diagnostic_used!(E0424);
            err.code(DiagnosticId::Error("E0424".into()));
            err.span_label(span, match source {
                PathSource::Pat => {
                    format!("`self` value is a keyword \
                             and may not be bound to \
                             variables or shadowed")
                }
                _ => {
                    format!("`self` value is a keyword \
                             only available in methods \
                             with `self` parameter")
                }
            });
            return (err, Vec::new());
        }

        // Try to lookup name in more relaxed fashion for better error reporting.
        let ident = path.last().unwrap().ident;
        let candidates = self.lookup_import_candidates(ident, ns, is_expected);
        if candidates.is_empty() && is_expected(Def::Enum(DefId::local(CRATE_DEF_INDEX))) {
            let enum_candidates =
                self.lookup_import_candidates(ident, ns, is_enum_variant);
            let mut enum_candidates = enum_candidates.iter()
                .map(|suggestion| {
                    import_candidate_to_enum_paths(&suggestion)
                }).collect::<Vec<_>>();
            enum_candidates.sort();

            if !enum_candidates.is_empty() {
                // Contextualize for E0412 "cannot find type", but don't belabor the point
                // (that it's a variant) for E0573 "expected type, found variant".
                let preamble = if def.is_none() {
                    let others = match enum_candidates.len() {
                        1 => String::new(),
                        2 => " and 1 other".to_owned(),
                        n => format!(" and {} others", n)
                    };
                    format!("there is an enum variant `{}`{}; ",
                            enum_candidates[0].0, others)
                } else {
