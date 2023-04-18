File_Code/nushell/617341f8d5/deserializer/deserializer_after.rs --- 1/2 --- Rust
                                                                                                                                                             7 use nu_source::Span;

File_Code/nushell/617341f8d5/deserializer/deserializer_after.rs --- 2/2 --- Rust
                                                                                                                                                           405                 Value {
                                                                                                                                                           406                     value: UntaggedValue::Primitive(Primitive::String(path)),
                                                                                                                                                           407                     ..
                                                                                                                                                           408                 } => {
                                                                                                                                                           409                     let s = path.spanned(Span::unknown());
                                                                                                                                                           410                     ColumnPath::build(&s)
                                                                                                                                                           411                 }

