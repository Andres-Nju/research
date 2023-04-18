File_Code/rust/e2d1d667e2/alloc/alloc_after.rs --- 1/3 --- Rust
1 use crate::alloc::{self, GlobalAlloc, Layout, System};                                                                                                     1 use crate::alloc::{GlobalAlloc, Layout, System};

File_Code/rust/e2d1d667e2/alloc/alloc_after.rs --- 2/3 --- Rust
39     alloc::alloc(Layout::from_size_align_unchecked(size, align))                                                                                          39     crate::alloc::alloc(Layout::from_size_align_unchecked(size, align))

File_Code/rust/e2d1d667e2/alloc/alloc_after.rs --- 3/3 --- Rust
45     alloc::dealloc(ptr, Layout::from_size_align_unchecked(size, align))                                                                                   45     crate::alloc::dealloc(ptr, Layout::from_size_align_unchecked(size, align))

