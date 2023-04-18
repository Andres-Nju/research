    fn visit_ty(&mut self, ty: &'a Ty) {
        match ty.node {
            TyKind::BareFn(ref bfty) => {
                self.check_decl_no_pat(&bfty.decl, |span, _| {
                    struct_span_err!(self.session, span, E0561,
                                     "patterns aren't allowed in function pointer types").emit();
                });
                self.check_late_bound_lifetime_defs(&bfty.generic_params);
            }
            TyKind::TraitObject(ref bounds, ..) => {
                let mut any_lifetime_bounds = false;
                for bound in bounds {
                    if let RegionTyParamBound(ref lifetime) = *bound {
                        if any_lifetime_bounds {
                            span_err!(self.session, lifetime.span, E0226,
                                      "only a single explicit lifetime bound is permitted");
                            break;
                        }
                        any_lifetime_bounds = true;
                    }
                }
                self.no_questions_in_bounds(bounds, "trait object types", false);
            }
            TyKind::ImplTrait(ref bounds) => {
                if !bounds.iter()
                          .any(|b| if let TraitTyParamBound(..) = *b { true } else { false }) {
                    self.err_handler().span_err(ty.span, "at least one trait must be specified");
                }
            }
            _ => {}
        }

        visit::walk_ty(self, ty)
    }

    fn visit_use_tree(&mut self, use_tree: &'a UseTree, id: NodeId, _nested: bool) {
        // Check if the path in this `use` is not generic, such as `use foo::bar<T>;` While this
        // can't happen normally thanks to the parser, a generic might sneak in if the `use` is
        // built using a macro.
        //
        // macro_use foo {
        //     ($p:path) => { use $p; }
        // }
        // foo!(bar::baz<T>);
        use_tree.prefix.segments.iter().find(|segment| {
            segment.parameters.is_some()
        }).map(|segment| {
            self.err_handler().span_err(segment.parameters.as_ref().unwrap().span(),
                                        "generic arguments in import path");
        });

        visit::walk_use_tree(self, use_tree, id);
    }

    fn visit_label(&mut self, label: &'a Label) {
        self.check_label(label.ident, label.span);
        visit::walk_label(self, label);
    }

    fn visit_lifetime(&mut self, lifetime: &'a Lifetime) {
        self.check_lifetime(lifetime);
        visit::walk_lifetime(self, lifetime);
    }

    fn visit_item(&mut self, item: &'a Item) {
        match item.node {
            ItemKind::Impl(unsafety, polarity, _, _, Some(..), ref ty, ref impl_items) => {
                self.invalid_visibility(&item.vis, None);
                if ty.node == TyKind::Err {
                    self.err_handler()
                        .struct_span_err(item.span, "`impl Trait for .. {}` is an obsolete syntax")
                        .help("use `auto trait Trait {}` instead").emit();
                }
                if unsafety == Unsafety::Unsafe && polarity == ImplPolarity::Negative {
                    span_err!(self.session, item.span, E0198, "negative impls cannot be unsafe");
                }
                for impl_item in impl_items {
                    self.invalid_visibility(&impl_item.vis, None);
                    if let ImplItemKind::Method(ref sig, _) = impl_item.node {
                        self.check_trait_fn_not_const(sig.constness);
                    }
                }
            }
            ItemKind::Impl(unsafety, polarity, defaultness, _, None, _, _) => {
                self.invalid_visibility(&item.vis,
                                        Some("place qualifiers on individual impl items instead"));
                if unsafety == Unsafety::Unsafe {
                    span_err!(self.session, item.span, E0197, "inherent impls cannot be unsafe");
                }
                if polarity == ImplPolarity::Negative {
                    self.err_handler().span_err(item.span, "inherent impls cannot be negative");
                }
                if defaultness == Defaultness::Default {
                    self.err_handler()
                        .struct_span_err(item.span, "inherent impls cannot be default")
                        .help("maybe a missing `for` keyword?").emit();
                }
            }
            ItemKind::ForeignMod(..) => {
                self.invalid_visibility(
                    &item.vis,
                    Some("place qualifiers on individual foreign items instead"),
                );
            }
            ItemKind::Enum(ref def, _) => {
                for variant in &def.variants {
                    self.invalid_non_exhaustive_attribute(variant);
                    for field in variant.node.data.fields() {
                        self.invalid_visibility(&field.vis, None);
                    }
                }
            }
            ItemKind::Trait(is_auto, _, ref generics, ref bounds, ref trait_items) => {
                if is_auto == IsAuto::Yes {
                    // Auto traits cannot have generics, super traits nor contain items.
                    if generics.is_parameterized() {
                        struct_span_err!(self.session, item.span, E0567,
                                        "auto traits cannot have generic parameters").emit();
                    }
                    if !bounds.is_empty() {
                        struct_span_err!(self.session, item.span, E0568,
                                        "auto traits cannot have super traits").emit();
                    }
                    if !trait_items.is_empty() {
                        struct_span_err!(self.session, item.span, E0380,
                                "auto traits cannot have methods or associated items").emit();
                    }
                }
                self.no_questions_in_bounds(bounds, "supertraits", true);
                for trait_item in trait_items {
                    if let TraitItemKind::Method(ref sig, ref block) = trait_item.node {
                        self.check_trait_fn_not_const(sig.constness);
                        if block.is_none() {
                            self.check_decl_no_pat(&sig.decl, |span, mut_ident| {
                                if mut_ident {
                                    self.session.buffer_lint(
                                        lint::builtin::PATTERNS_IN_FNS_WITHOUT_BODY,
                                        trait_item.id, span,
                                        "patterns aren't allowed in methods without bodies");
                                } else {
                                    struct_span_err!(self.session, span, E0642,
                                        "patterns aren't allowed in methods without bodies").emit();
                                }
                            });
                        }
                    }
                }
            }
            ItemKind::TraitAlias(Generics { ref params, .. }, ..) => {
                for param in params {
                    if let GenericParam::Type(TyParam {
                        ref bounds,
                        ref default,
                        span,
                        ..
                    }) = *param
                    {
                        if !bounds.is_empty() {
                            self.err_handler().span_err(span,
                                                        "type parameters on the left side of a \
                                                         trait alias cannot be bounded");
                        }
                        if !default.is_none() {
                            self.err_handler().span_err(span,
                                                        "type parameters on the left side of a \
                                                         trait alias cannot have defaults");
                        }
                    }
                }
            }
            ItemKind::Mod(_) => {
                // Ensure that `path` attributes on modules are recorded as used (c.f. #35584).
                attr::first_attr_value_str_by_name(&item.attrs, "path");
                if attr::contains_name(&item.attrs, "warn_directory_ownership") {
                    let lint = lint::builtin::LEGACY_DIRECTORY_OWNERSHIP;
                    let msg = "cannot declare a new module at this location";
                    self.session.buffer_lint(lint, item.id, item.span, msg);
                }
            }
            ItemKind::Union(ref vdata, _) => {
                if !vdata.is_struct() {
                    self.err_handler().span_err(item.span,
                                                "tuple and unit unions are not permitted");
                }
                if vdata.fields().len() == 0 {
                    self.err_handler().span_err(item.span,
                                                "unions cannot have zero fields");
                }
            }
            _ => {}
        }

        visit::walk_item(self, item)
    }

    fn visit_foreign_item(&mut self, fi: &'a ForeignItem) {
        match fi.node {
            ForeignItemKind::Fn(ref decl, _) => {
                self.check_decl_no_pat(decl, |span, _| {
                    struct_span_err!(self.session, span, E0130,
                                     "patterns aren't allowed in foreign function declarations")
                        .span_label(span, "pattern not allowed in foreign function").emit();
                });
            }
            ForeignItemKind::Static(..) | ForeignItemKind::Ty => {}
        }

        visit::walk_foreign_item(self, fi)
    }

    fn visit_vis(&mut self, vis: &'a Visibility) {
        match vis.node {
            VisibilityKind::Restricted { ref path, .. } => {
                path.segments.iter().find(|segment| segment.parameters.is_some()).map(|segment| {
                    self.err_handler().span_err(segment.parameters.as_ref().unwrap().span(),
                                                "generic arguments in visibility path");
                });
            }
            _ => {}
        }

        visit::walk_vis(self, vis)
    }
