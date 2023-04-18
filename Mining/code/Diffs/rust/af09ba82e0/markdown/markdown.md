File_Code/rust/af09ba82e0/markdown/markdown_after.rs --- 1/3 --- Rust
374                     let reference = format!("<sup id=\"supref{0}\"><a href=\"#ref{0}\">{0}\                                                              374                     let reference = format!("<sup id=\"fnref{0}\"><a href=\"#fn{0}\">{0}\
375                                              </a></sup>",                                                                                                375                                              </a></sup>",

File_Code/rust/af09ba82e0/markdown/markdown_after.rs --- 2/3 --- Rust
397                             write!(ret, "<li id=\"ref{}\">", id).unwrap();                                                                               397                             write!(ret, "<li id=\"fn{}\">", id).unwrap();

File_Code/rust/af09ba82e0/markdown/markdown_after.rs --- 3/3 --- Rust
405                                    "&nbsp;<a href=\"#supref{}\" rev=\"footnote\">↩</a>",                                                                 405                                    "&nbsp;<a href=\"#fnref{}\" rev=\"footnote\">↩</a>",

