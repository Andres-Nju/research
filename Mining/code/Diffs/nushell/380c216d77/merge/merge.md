File_Code/nushell/380c216d77/merge/merge_after.rs --- Rust
36                 "block",                                                                                                                                  36                 "value",
37                 // Both this and `update` should have a shape more like <record> | <table> than just <any>. -Leon 2022-10-27                              37                 // Both this and `update` should have a shape more like <record> | <table> than just <any>. -Leon 2022-10-27
38                 SyntaxShape::Any,                                                                                                                         38                 SyntaxShape::Any,
39                 "the new value to merge with, or a block that produces it",                                                                               39                 "the new value to merge with",

