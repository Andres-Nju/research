File_Code/rust/7adb20e4cd/reachable/reachable_after.rs --- 1/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
36         hir::ItemKind::Fn(_, header, ..) if header.constness == hir::Constness::Const => {                                                                36         hir::ItemKind::Fn(_, header, ..) if header.is_const() => {

File_Code/rust/7adb20e4cd/reachable/reachable_after.rs --- 2/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
59         if method_sig.header.constness == hir::Constness::Const {                                                                                         59         if method_sig.header.is_const() {

