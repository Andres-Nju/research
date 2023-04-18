fn item_might_be_inlined(tcx: TyCtxt<'tcx>, item: &hir::Item, attrs: CodegenFnAttrs) -> bool {
    if attrs.requests_inline() {
        return true
    }

    match item.node {
        hir::ItemKind::Fn(_, header, ..) if header.constness == hir::Constness::Const => {
            return true;
        }
        hir::ItemKind::Impl(..) |
        hir::ItemKind::Fn(..) => {
            let generics = tcx.generics_of(tcx.hir().local_def_id(item.hir_id));
            generics.requires_monomorphization(tcx)
        }
        _ => false,
    }
}

fn method_might_be_inlined(
    tcx: TyCtxt<'_>,
    impl_item: &hir::ImplItem,
    impl_src: DefId,
) -> bool {
    let codegen_fn_attrs = tcx.codegen_fn_attrs(impl_item.hir_id.owner_def_id());
    let generics = tcx.generics_of(tcx.hir().local_def_id(impl_item.hir_id));
    if codegen_fn_attrs.requests_inline() || generics.requires_monomorphization(tcx) {
        return true
    }
    if let hir::ImplItemKind::Method(method_sig, _) = &impl_item.node {
        if method_sig.header.constness == hir::Constness::Const {
            return true
        }
    }
    if let Some(impl_hir_id) = tcx.hir().as_local_hir_id(impl_src) {
        match tcx.hir().find(impl_hir_id) {
            Some(Node::Item(item)) =>
                item_might_be_inlined(tcx, &item, codegen_fn_attrs),
            Some(..) | None =>
                span_bug!(impl_item.span, "impl did is not an item")
        }
    } else {
        span_bug!(impl_item.span, "found a foreign impl as a parent of a local method")
    }
}
