    fn construct_async_arguments(&mut self, asyncness: &mut Spanned<IsAsync>, decl: &mut FnDecl) {
        // FIXME(davidtwco): This function should really live in the HIR lowering but because
        // the types constructed here need to be used in parts of resolve so that the correct
        // locals are considered upvars, it is currently easier for it to live here in the parser,
        // where it can be constructed once.
        if let IsAsync::Async { ref mut arguments, .. } = asyncness.node {
            for (index, input) in decl.inputs.iter_mut().enumerate() {
                let id = ast::DUMMY_NODE_ID;
                let span = input.pat.span;

                // Construct a name for our temporary argument.
                let name = format!("__arg{}", index);
                let ident = Ident::from_str(&name).gensym();

                // Check if this is a ident pattern, if so, we can optimize and avoid adding a
                // `let <pat> = __argN;` statement, instead just adding a `let <pat> = <pat>;`
                // statement.
                let (binding_mode, ident, is_simple_pattern) = match input.pat.node {
                    PatKind::Ident(binding_mode @ BindingMode::ByValue(_), ident, _) => {
                        // Simple patterns like this don't have a generated argument, but they are
                        // moved into the closure with a statement, so any `mut` bindings on the
                        // argument will be unused. This binding mode can't be removed, because
                        // this would affect the input to procedural macros, but they can have
                        // their span marked as being the result of a compiler desugaring so
                        // that they aren't linted against.
                        input.pat.span = self.sess.source_map().mark_span_with_reason(
                            CompilerDesugaringKind::Async, span, None);

                        (binding_mode, ident, true)
                    }
                    _ => (BindingMode::ByValue(Mutability::Mutable), ident, false),
                };

                // Construct an argument representing `__argN: <ty>` to replace the argument of the
                // async function if it isn't a simple pattern.
                let arg = if is_simple_pattern {
                    None
                } else {
                    Some(Arg {
                        ty: input.ty.clone(),
                        id,
                        pat: P(Pat {
                            id,
                            node: PatKind::Ident(
                                BindingMode::ByValue(Mutability::Immutable), ident, None,
                            ),
                            span,
                        }),
                        source: ArgSource::AsyncFn(input.pat.clone()),
                    })
                };

                // Construct a `let __argN = __argN;` statement to insert at the top of the
                // async closure. This makes sure that the argument is captured by the closure and
                // that the drop order is correct.
                let move_local = Local {
                    pat: P(Pat {
                        id,
                        node: PatKind::Ident(binding_mode, ident, None),
                        span,
                    }),
                    // We explicitly do not specify the type for this statement. When the user's
                    // argument type is `impl Trait` then this would require the
                    // `impl_trait_in_bindings` feature to also be present for that same type to
                    // be valid in this binding. At the time of writing (13 Mar 19),
                    // `impl_trait_in_bindings` is not stable.
                    ty: None,
                    init: Some(P(Expr {
                        id,
                        node: ExprKind::Path(None, ast::Path {
                            span,
                            segments: vec![PathSegment { ident, id, args: None }],
                        }),
                        span,
                        attrs: ThinVec::new(),
                    })),
                    id,
                    span,
                    attrs: ThinVec::new(),
                    source: LocalSource::AsyncFn,
                };

                // Construct a `let <pat> = __argN;` statement to insert at the top of the
                // async closure if this isn't a simple pattern.
                let pat_stmt = if is_simple_pattern {
                    None
                } else {
                    Some(Stmt {
                        id,
                        node: StmtKind::Local(P(Local {
                            pat: input.pat.clone(),
                            ..move_local.clone()
                        })),
                        span,
                    })
                };

                let move_stmt = Stmt { id, node: StmtKind::Local(P(move_local)), span };
                arguments.push(AsyncArgument { ident, arg, pat_stmt, move_stmt });
            }
        }
    }
