File_Code/rust-analyzer/b168104d95/hover/hover_after.rs --- 1/2 --- Rust
230             node.name()?.syntax().text().push_to(&mut string);                                                                                           230             string.push_str(node.name()?.text().as_str());

File_Code/rust-analyzer/b168104d95/hover/hover_after.rs --- 2/2 --- Rust
244             .visit(|node: &ast::EnumVariant| Some(node.name()?.syntax().text().to_string()))                                                             244             .visit(|node: &ast::EnumVariant| Some(node.name()?.text().to_string()))

