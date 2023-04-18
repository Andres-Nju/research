    fn smart_resolve_context_dependent_help(
        &mut self,
        err: &mut DiagnosticBuilder<'a>,
        span: Span,
        source: PathSource<'_>,
        def: Def,
        path_str: &str,
        fallback_label: &str,
    ) -> bool {
        let ns = source.namespace();
        let is_expected = &|def| source.is_expected(def);

        match (def, source) {
            (Def::Macro(..), _) => {
                err.span_suggestion(
                    span,
                    "use `!` to invoke the macro",
                    format!("{}!", path_str),
                    Applicability::MaybeIncorrect,
                );
            }
            (Def::TyAlias(..), PathSource::Trait(_)) => {
                err.span_label(span, "type aliases cannot be used as traits");
                if nightly_options::is_nightly_build() {
                    err.note("did you mean to use a trait alias?");
                }
            }
            (Def::Mod(..), PathSource::Expr(Some(parent))) => match parent.node {
                ExprKind::Field(_, ident) => {
                    err.span_suggestion(
                        parent.span,
                        "use the path separator to refer to an item",
                        format!("{}::{}", path_str, ident),
                        Applicability::MaybeIncorrect,
                    );
                }
                ExprKind::MethodCall(ref segment, ..) => {
                    let span = parent.span.with_hi(segment.ident.span.hi());
                    err.span_suggestion(
                        span,
                        "use the path separator to refer to an item",
                        format!("{}::{}", path_str, segment.ident),
                        Applicability::MaybeIncorrect,
                    );
                }
                _ => return false,
            },
            (Def::Enum(..), PathSource::TupleStruct)
                | (Def::Enum(..), PathSource::Expr(..))  => {
                if let Some(variants) = self.collect_enum_variants(def) {
                    err.note(&format!("did you mean to use one \
                                       of the following variants?\n{}",
                        variants.iter()
                            .map(|suggestion| path_names_to_string(suggestion))
                            .map(|suggestion| format!("- `{}`", suggestion))
                            .collect::<Vec<_>>()
                            .join("\n")));
                } else {
                    err.note("did you mean to use one of the enum's variants?");
                }
            },
            (Def::Struct(def_id), _) if ns == ValueNS => {
                if let Some((ctor_def, ctor_vis))
                        = self.struct_constructors.get(&def_id).cloned() {
                    let accessible_ctor = self.is_accessible(ctor_vis);
                    if is_expected(ctor_def) && !accessible_ctor {
                        err.span_label(span, format!("constructor is not visible \
                                                      here due to private fields"));
                    }
                } else {
                    // HACK(estebank): find a better way to figure out that this was a
                    // parser issue where a struct literal is being used on an expression
                    // where a brace being opened means a block is being started. Look
                    // ahead for the next text to see if `span` is followed by a `{`.
                    let sm = self.session.source_map();
                    let mut sp = span;
                    loop {
                        sp = sm.next_point(sp);
                        match sm.span_to_snippet(sp) {
                            Ok(ref snippet) => {
                                if snippet.chars().any(|c| { !c.is_whitespace() }) {
                                    break;
                                }
                            }
                            _ => break,
                        }
                    }
                    let followed_by_brace = match sm.span_to_snippet(sp) {
                        Ok(ref snippet) if snippet == "{" => true,
                        _ => false,
                    };
                    // In case this could be a struct literal that needs to be surrounded
                    // by parenthesis, find the appropriate span.
                    let mut i = 0;
                    let mut closing_brace = None;
                    loop {
                        sp = sm.next_point(sp);
                        match sm.span_to_snippet(sp) {
                            Ok(ref snippet) => {
                                if snippet == "}" {
                                    let sp = span.to(sp);
                                    if let Ok(snippet) = sm.span_to_snippet(sp) {
                                        closing_brace = Some((sp, snippet));
                                    }
                                    break;
                                }
                            }
                            _ => break,
                        }
                        i += 1;
                        // The bigger the span, the more likely we're incorrect --
                        // bound it to 100 chars long.
                        if i > 100 {
                            break;
                        }
                    }
                    match source {
                        PathSource::Expr(Some(parent)) => {
                            match parent.node {
                                ExprKind::MethodCall(ref path_assignment, _)  => {
                                    err.span_suggestion(
                                        sm.start_point(parent.span)
                                            .to(path_assignment.ident.span),
                                        "use `::` to access an associated function",
                                        format!("{}::{}",
                                                path_str,
                                                path_assignment.ident),
                                        Applicability::MaybeIncorrect
                                    );
                                },
                                _ => {
                                    err.span_label(
                                        span,
                                        format!("did you mean `{} {{ /* fields */ }}`?",
                                                path_str),
                                    );
                                },
                            }
                        },
                        PathSource::Expr(None) if followed_by_brace == true => {
                            if let Some((sp, snippet)) = closing_brace {
                                err.span_suggestion(
                                    sp,
                                    "surround the struct literal with parenthesis",
                                    format!("({})", snippet),
                                    Applicability::MaybeIncorrect,
                                );
                            } else {
                                err.span_label(
                                    span,
                                    format!("did you mean `({} {{ /* fields */ }})`?",
                                            path_str),
                                );
                            }
                        },
                        _ => {
                            err.span_label(
                                span,
                                format!("did you mean `{} {{ /* fields */ }}`?",
                                        path_str),
                            );
                        },
                    }
                }
            }
            (Def::Union(..), _) |
            (Def::Variant(..), _) |
            (Def::VariantCtor(_, CtorKind::Fictive), _) if ns == ValueNS => {
                err.span_label(span, format!("did you mean `{} {{ /* fields */ }}`?",
                                             path_str));
            }
            (Def::SelfTy(..), _) if ns == ValueNS => {
                err.span_label(span, fallback_label);
                err.note("can't use `Self` as a constructor, you must use the \
                          implemented struct");
            }
            (Def::TyAlias(_), _) | (Def::AssociatedTy(..), _) if ns == ValueNS => {
                err.note("can't use a type alias as a constructor");
            }
            _ => return false,
        }
        true
    }
}

impl<'a, 'b:'a> ImportResolver<'a, 'b> {
    /// Adds suggestions for a path that cannot be resolved.
    pub(crate) fn make_path_suggestion(
        &mut self,
        span: Span,
        mut path: Vec<Segment>,
        parent_scope: &ParentScope<'b>,
    ) -> Option<(Vec<Segment>, Option<String>)> {
        debug!("make_path_suggestion: span={:?} path={:?}", span, path);

        match (path.get(0), path.get(1)) {
            // `{{root}}::ident::...` on both editions.
            // On 2015 `{{root}}` is usually added implicitly.
            (Some(fst), Some(snd)) if fst.ident.name == keywords::PathRoot.name() &&
                                      !snd.ident.is_path_segment_keyword() => {}
            // `ident::...` on 2018.
            (Some(fst), _) if fst.ident.span.rust_2018() &&
                              !fst.ident.is_path_segment_keyword() => {
                // Insert a placeholder that's later replaced by `self`/`super`/etc.
                path.insert(0, Segment::from_ident(keywords::Invalid.ident()));
            }
            _ => return None,
        }

        self.make_missing_self_suggestion(span, path.clone(), parent_scope)
            .or_else(|| self.make_missing_crate_suggestion(span, path.clone(), parent_scope))
            .or_else(|| self.make_missing_super_suggestion(span, path.clone(), parent_scope))
            .or_else(|| self.make_external_crate_suggestion(span, path, parent_scope))
    }

    /// Suggest a missing `self::` if that resolves to an correct module.
    ///
    /// ```
    ///    |
    /// LL | use foo::Bar;
    ///    |     ^^^ did you mean `self::foo`?
    /// ```
    fn make_missing_self_suggestion(
        &mut self,
        span: Span,
        mut path: Vec<Segment>,
        parent_scope: &ParentScope<'b>,
    ) -> Option<(Vec<Segment>, Option<String>)> {
        // Replace first ident with `self` and check if that is valid.
        path[0].ident.name = keywords::SelfLower.name();
        let result = self.resolve_path(&path, None, parent_scope, false, span, CrateLint::No);
        debug!("make_missing_self_suggestion: path={:?} result={:?}", path, result);
        if let PathResult::Module(..) = result {
            Some((path, None))
        } else {
            None
        }
    }

    /// Suggests a missing `crate::` if that resolves to an correct module.
    ///
    /// ```
    ///    |
    /// LL | use foo::Bar;
    ///    |     ^^^ did you mean `crate::foo`?
    /// ```
    fn make_missing_crate_suggestion(
        &mut self,
        span: Span,
        mut path: Vec<Segment>,
        parent_scope: &ParentScope<'b>,
    ) -> Option<(Vec<Segment>, Option<String>)> {
        // Replace first ident with `crate` and check if that is valid.
        path[0].ident.name = keywords::Crate.name();
        let result = self.resolve_path(&path, None, parent_scope, false, span, CrateLint::No);
        debug!("make_missing_crate_suggestion:  path={:?} result={:?}", path, result);
        if let PathResult::Module(..) = result {
            Some((
                path,
                Some(
                    "`use` statements changed in Rust 2018; read more at \
                     <https://doc.rust-lang.org/edition-guide/rust-2018/module-system/path-\
                     clarity.html>".to_string()
                ),
            ))
        } else {
            None
        }
