    fn resolve_expr(&mut self, expr: &Expr, parent: Option<&Expr>) {
        // First, record candidate traits for this expression if it could
        // result in the invocation of a method call.

        self.record_candidate_traits_for_expr_if_necessary(expr);

        // Next, resolve the node.
        match expr.node {
            ExprKind::Path(ref maybe_qself, ref path) => {
                // This is a local path in the value namespace. Walk through
                // scopes looking for it.
                if let Some(path_res) = self.resolve_possibly_assoc_item(expr.id,
                                                            maybe_qself.as_ref(), path, ValueNS) {
                    // Check if struct variant
                    let is_struct_variant = if let Def::Variant(_, variant_id) = path_res.base_def {
                        self.structs.contains_key(&variant_id)
                    } else {
                        false
                    };
                    if is_struct_variant {
                        let _ = self.structs.contains_key(&path_res.base_def.def_id());
                        let path_name = path_names_to_string(path, 0);

                        let mut err = resolve_struct_error(self,
                                        expr.span,
                                        ResolutionError::StructVariantUsedAsFunction(&path_name));

                        let msg = format!("did you mean to write: `{} {{ /* fields */ }}`?",
                                          path_name);
                        if self.emit_errors {
                            err.help(&msg);
                        } else {
                            err.span_help(expr.span, &msg);
                        }
                        err.emit();
                        self.record_def(expr.id, err_path_resolution());
                    } else {
                        // Write the result into the def map.
                        debug!("(resolving expr) resolved `{}`",
                               path_names_to_string(path, 0));

                        // Partial resolutions will need the set of traits in scope,
                        // so they can be completed during typeck.
                        if path_res.depth != 0 {
                            let method_name = path.segments.last().unwrap().identifier.name;
                            let traits = self.get_traits_containing_item(method_name);
                            self.trait_map.insert(expr.id, traits);
                        }

                        self.record_def(expr.id, path_res);
                    }
                } else {
                    // Be helpful if the name refers to a struct
                    // (The pattern matching def_tys where the id is in self.structs
                    // matches on regular structs while excluding tuple- and enum-like
                    // structs, which wouldn't result in this error.)
                    let path_name = path_names_to_string(path, 0);
                    let type_res = self.with_no_errors(|this| {
                        this.resolve_path(expr.id, path, 0, TypeNS)
                    });

                    self.record_def(expr.id, err_path_resolution());

                    if let Ok(Def::Struct(..)) = type_res.map(|r| r.base_def) {
                        let error_variant =
                            ResolutionError::StructVariantUsedAsFunction(&path_name);
                        let mut err = resolve_struct_error(self, expr.span, error_variant);

                        let msg = format!("did you mean to write: `{} {{ /* fields */ }}`?",
                                          path_name);

                        if self.emit_errors {
                            err.help(&msg);
                        } else {
                            err.span_help(expr.span, &msg);
                        }
                        err.emit();
                    } else {
                        // Keep reporting some errors even if they're ignored above.
                        if let Err(true) = self.resolve_path(expr.id, path, 0, ValueNS) {
                            // `resolve_path` already reported the error
                        } else {
                            let mut method_scope = false;
                            let mut is_static = false;
                            self.value_ribs.iter().rev().all(|rib| {
                                method_scope = match rib.kind {
                                    MethodRibKind(is_static_) => {
                                        is_static = is_static_;
                                        true
                                    }
                                    ItemRibKind | ConstantItemRibKind => false,
                                    _ => return true, // Keep advancing
                                };
                                false // Stop advancing
                            });

                            if method_scope &&
                                    &path_name[..] == keywords::SelfValue.name().as_str() {
                                resolve_error(self,
                                              expr.span,
                                              ResolutionError::SelfNotAvailableInStaticMethod);
                            } else {
                                let last_name = path.segments.last().unwrap().identifier.name;
                                let (mut msg, is_field) =
                                    match self.find_fallback_in_self_type(last_name) {
                                    NoSuggestion => {
                                        // limit search to 5 to reduce the number
                                        // of stupid suggestions
                                        (match self.find_best_match(&path_name) {
                                            SuggestionType::Macro(s) => {
                                                format!("the macro `{}`", s)
                                            }
                                            SuggestionType::Function(s) => format!("`{}`", s),
                                            SuggestionType::NotFound => "".to_string(),
                                        }, false)
                                    }
                                    Field => {
                                        (if is_static && method_scope {
                                            "".to_string()
                                        } else {
                                            format!("`self.{}`", path_name)
                                        }, true)
                                    }
                                    TraitItem => (format!("to call `self.{}`", path_name), false),
                                    TraitMethod(path_str) =>
                                        (format!("to call `{}::{}`", path_str, path_name), false),
                                };

                                let mut context =  UnresolvedNameContext::Other;
                                let mut def = Def::Err;
                                if !msg.is_empty() {
                                    msg = format!(". Did you mean {}?", msg);
                                } else {
                                    // we check if this a module and if so, we display a help
                                    // message
                                    let name_path = path.segments.iter()
                                                        .map(|seg| seg.identifier.name)
                                                        .collect::<Vec<_>>();

                                    match self.resolve_module_path(&name_path[..],
                                                                   UseLexicalScope,
                                                                   expr.span) {
                                        Success(e) => {
                                            if let Some(def_type) = e.def {
                                                def = def_type;
                                            }
                                            context = UnresolvedNameContext::PathIsMod(parent);
                                        },
                                        _ => {},
                                    };
                                }

                                resolve_error(self,
                                              expr.span,
                                              ResolutionError::UnresolvedName {
                                                  path: &path_name,
                                                  message: &msg,
                                                  context: context,
                                                  is_static_method: method_scope && is_static,
                                                  is_field: is_field,
                                                  def: def,
                                              });
                            }
                        }
                    }
                }

                visit::walk_expr(self, expr);
            }

            ExprKind::Struct(ref path, _, _) => {
                // Resolve the path to the structure it goes to. We don't
                // check to ensure that the path is actually a structure; that
                // is checked later during typeck.
                match self.resolve_path(expr.id, path, 0, TypeNS) {
                    Ok(definition) => self.record_def(expr.id, definition),
                    Err(true) => self.record_def(expr.id, err_path_resolution()),
                    Err(false) => {
                        debug!("(resolving expression) didn't find struct def",);

                        resolve_error(self,
                                      path.span,
                                      ResolutionError::DoesNotNameAStruct(
                                                                &path_names_to_string(path, 0))
                                     );
                        self.record_def(expr.id, err_path_resolution());
                    }
                }

                visit::walk_expr(self, expr);
            }

            ExprKind::Loop(_, Some(label)) | ExprKind::While(_, _, Some(label)) => {
                self.with_label_rib(|this| {
                    let def = Def::Label(expr.id);

                    {
                        let rib = this.label_ribs.last_mut().unwrap();
                        rib.bindings.insert(mtwt::resolve(label.node), def);
                    }

                    visit::walk_expr(this, expr);
                })
            }

            ExprKind::Break(Some(label)) | ExprKind::Continue(Some(label)) => {
                match self.search_label(mtwt::resolve(label.node)) {
                    None => {
                        self.record_def(expr.id, err_path_resolution());
                        resolve_error(self,
                                      label.span,
                                      ResolutionError::UndeclaredLabel(&label.node.name.as_str()))
                    }
                    Some(def @ Def::Label(_)) => {
                        // Since this def is a label, it is never read.
                        self.record_def(expr.id, PathResolution::new(def))
                    }
                    Some(_) => {
                        span_bug!(expr.span, "label wasn't mapped to a label def!")
                    }
                }
            }

            ExprKind::IfLet(ref pattern, ref subexpression, ref if_block, ref optional_else) => {
                self.visit_expr(subexpression);

                self.value_ribs.push(Rib::new(NormalRibKind));
                self.resolve_pattern(pattern, PatternSource::IfLet, &mut HashMap::new());
                self.visit_block(if_block);
                self.value_ribs.pop();

                optional_else.as_ref().map(|expr| self.visit_expr(expr));
            }

            ExprKind::WhileLet(ref pattern, ref subexpression, ref block, label) => {
                self.visit_expr(subexpression);
                self.value_ribs.push(Rib::new(NormalRibKind));
                self.resolve_pattern(pattern, PatternSource::WhileLet, &mut HashMap::new());

                self.resolve_labeled_block(label.map(|l| l.node), expr.id, block);

                self.value_ribs.pop();
            }

            ExprKind::ForLoop(ref pattern, ref subexpression, ref block, label) => {
                self.visit_expr(subexpression);
                self.value_ribs.push(Rib::new(NormalRibKind));
                self.resolve_pattern(pattern, PatternSource::For, &mut HashMap::new());

                self.resolve_labeled_block(label.map(|l| l.node), expr.id, block);

                self.value_ribs.pop();
            }

            ExprKind::Field(ref subexpression, _) => {
                self.resolve_expr(subexpression, Some(expr));
            }
            ExprKind::MethodCall(_, ref types, ref arguments) => {
                let mut arguments = arguments.iter();
                self.resolve_expr(arguments.next().unwrap(), Some(expr));
                for argument in arguments {
                    self.resolve_expr(argument, None);
                }
                for ty in types.iter() {
                    self.visit_ty(ty);
                }
            }

            _ => {
                visit::walk_expr(self, expr);
            }
        }
    }
