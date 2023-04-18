File_Code/nushell/55374ee54f/alias/alias_after.rs --- Rust
26             .required("block", SyntaxShape::Block, "the block to run on each row")                                                                        26             .required(
..                                                                                                                                                           27                 "block",
..                                                                                                                                                           28                 SyntaxShape::Block,
..                                                                                                                                                           29                 "the block to run as the body of the alias",
..                                                                                                                                                           30             )
27     }                                                                                                                                                     31     }
28                                                                                                                                                           32 
29     fn usage(&self) -> &str {                                                                                                                             33     fn usage(&self) -> &str {
30         "Run a block on each row of the table."                                                                                                           34         "Define a shortcut for another command."

