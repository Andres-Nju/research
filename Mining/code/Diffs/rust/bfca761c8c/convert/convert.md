File_Code/rust/bfca761c8c/convert/convert_after.rs --- 1/2 --- Rust
74 /// - `AsRef` auto-dereference if the inner type is a reference or a mutable                                                                              74 /// - `AsRef` auto-dereferences if the inner type is a reference or a mutable
75 /// reference (eg: `foo.as_ref()` will work the same if `foo` has type `&mut Foo` or `&&mut Foo`)                                                         75 /// reference (e.g.: `foo.as_ref()` will work the same if `foo` has type `&mut Foo` or `&&mut Foo`)

File_Code/rust/bfca761c8c/convert/convert_after.rs --- 2/2 --- Rust
91 /// - `AsMut` auto-dereference if the inner type is a reference or a mutable                                                                              91 /// - `AsMut` auto-dereferences if the inner type is a reference or a mutable
92 /// reference (eg: `foo.as_ref()` will work the same if `foo` has type `&mut Foo` or `&&mut Foo`)                                                         92 /// reference (e.g.: `foo.as_ref()` will work the same if `foo` has type `&mut Foo` or `&&mut Foo`)

