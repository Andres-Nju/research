File_Code/rust-analyzer/d2b23599b6/lib/lib_after.rs --- 1/3 --- Rust
239     _hidden: (),                                                                                                                                         239     prev: bool,

File_Code/rust-analyzer/d2b23599b6/lib/lib_after.rs --- 2/3 --- Rust
244         IN_SCOPE.with(|slot| *slot.borrow_mut() = true);                                                                                                 244         let prev = IN_SCOPE.with(|slot| std::mem::replace(&mut *slot.borrow_mut(), true));
245         Scope { _hidden: () }                                                                                                                            245         Scope { prev }

File_Code/rust-analyzer/d2b23599b6/lib/lib_after.rs --- 3/3 --- Rust
254         IN_SCOPE.with(|slot| *slot.borrow_mut() = false);                                                                                                254         IN_SCOPE.with(|slot| *slot.borrow_mut() = self.prev);

