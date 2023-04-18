File_Code/nushell/08c0bf52bc/join/join_after.rs --- 1/4 --- Rust
7 use std::path::Path;                                                                                                                                       7 use std::path::{Path, PathBuf};

File_Code/nushell/08c0bf52bc/join/join_after.rs --- 2/4 --- Rust
13     append: Option<Tagged<String>>,                                                                                                                       13     append: Option<Tagged<PathBuf>>,

File_Code/nushell/08c0bf52bc/join/join_after.rs --- 3/4 --- Rust
32                 SyntaxShape::String,                                                                                                                      32                 SyntaxShape::FilePath,

File_Code/nushell/08c0bf52bc/join/join_after.rs --- 4/4 --- Rust
43         r#"Optionally, append an additional path to the result. It is designed to accept                                                                  43         r#"Optionally, append an additional path to the result. It is designed to accept
44 the output of 'path parse' and 'path split' subdommands."#                                                                                                44 the output of 'path parse' and 'path split' subcommands."#

