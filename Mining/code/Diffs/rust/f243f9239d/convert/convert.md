File_Code/rust/f243f9239d/convert/convert_after.rs --- Rust
144 /// - `AsMut` auto-dereferences if the inner type is a reference or a mutable                                                                            144 /// - `AsMut` auto-dereferences if the inner type is a mutable reference
145 ///   reference (e.g.: `foo.as_ref()` will work the same if `foo` has type                                                                               145 ///   (e.g.: `foo.as_mut()` will work the same if `foo` has type `&mut Foo`
146 ///   `&mut Foo` or `&&mut Foo`)                                                                                                                         146 ///   or `&mut &mut Foo`)

