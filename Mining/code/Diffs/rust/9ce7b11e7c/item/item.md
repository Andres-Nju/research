File_Code/rust/9ce7b11e7c/item/item_after.rs --- Rust
11 //! Walks the crate looking for items/impl-items/trait-items that have                                                                                      
12 //! either a `rustc_symbol_name` or `rustc_item_path` attribute and                                                                                         
13 //! generates an error giving, respectively, the symbol name or                                                                                             
14 //! item-path. This is used for unit testing the code that generates                                                                                        
15 //! paths etc in all kinds of annoying scenarios.                                                                                                           

