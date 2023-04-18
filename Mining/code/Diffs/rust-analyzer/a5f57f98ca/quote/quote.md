File_Code/rust-analyzer/a5f57f98ca/quote/quote_after.rs --- Rust
245             .into_iter()                                                                                                                                 245             fields.iter().map(|it| quote!(#it: self.#it.clone(), ).token_trees.clone()).flatten();

