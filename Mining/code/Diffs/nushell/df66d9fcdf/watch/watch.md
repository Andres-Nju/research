File_Code/nushell/df66d9fcdf/watch/watch_after.rs --- Rust
38             .required("block", SyntaxShape::Block, "A Nu block of code to run whenever a file changes. The block will be passed `operation`, `path`, and  38             .required("closure",
   `new_path` (for renames only) arguments in that order")                                                                                                   .. 
                                                                                                                                                             39             SyntaxShape::Closure(Some(vec![SyntaxShape::String, SyntaxShape::String, SyntaxShape::String])),
                                                                                                                                                             40                 "Some Nu code to run whenever a file changes. The closure will be passed `operation`, `path`, and `new_path` (for renames only) arguments
                                                                                                                                                                 in that order")

