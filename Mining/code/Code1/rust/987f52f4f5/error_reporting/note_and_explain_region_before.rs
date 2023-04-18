    pub fn note_and_explain_region(self,
                                   err: &mut DiagnosticBuilder,
                                   prefix: &str,
                                   region: &'tcx ty::Region,
                                   suffix: &str) {
        fn item_scope_tag(item: &hir::Item) -> &'static str {
            match item.node {
                hir::ItemImpl(..) => "impl",
                hir::ItemStruct(..) => "struct",
                hir::ItemUnion(..) => "union",
                hir::ItemEnum(..) => "enum",
                hir::ItemTrait(..) => "trait",
                hir::ItemFn(..) => "function body",
                _ => "item"
            }
        }

        fn trait_item_scope_tag(item: &hir::TraitItem) -> &'static str {
            match item.node {
                hir::MethodTraitItem(..) => "method body",
                hir::ConstTraitItem(..) |
                hir::TypeTraitItem(..) => "associated item"
            }
        }

        fn impl_item_scope_tag(item: &hir::ImplItem) -> &'static str {
            match item.node {
                hir::ImplItemKind::Method(..) => "method body",
                hir::ImplItemKind::Const(..) |
                hir::ImplItemKind::Type(_) => "associated item"
            }
        }

        fn explain_span<'a, 'gcx, 'tcx>(tcx: TyCtxt<'a, 'gcx, 'tcx>,
                                        heading: &str, span: Span)
                                        -> (String, Option<Span>) {
            let lo = tcx.sess.codemap().lookup_char_pos_adj(span.lo);
            (format!("the {} at {}:{}", heading, lo.line, lo.col.to_usize()),
             Some(span))
        }
