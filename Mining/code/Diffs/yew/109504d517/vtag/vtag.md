File_Code/yew/109504d517/vtag/vtag_after.rs --- Rust
355         self.childs.drain(..).for_each(|mut v| {                                                                                                         355         self.childs.drain(..).for_each(|mut child| {
356             v.detach(&node);                                                                                                                             356             child.detach(&node);

