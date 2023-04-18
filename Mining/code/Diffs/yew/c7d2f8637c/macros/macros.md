File_Code/yew/c7d2f8637c/macros/macros_after.rs --- Rust
                                                                                                                                                           182     ($stack:ident $($tail:tt)*) => {
                                                                                                                                                           183         compile_error!("You should use curly bracets for text nodes: <a>{ \"Link\" }</a>");
                                                                                                                                                           184     };

