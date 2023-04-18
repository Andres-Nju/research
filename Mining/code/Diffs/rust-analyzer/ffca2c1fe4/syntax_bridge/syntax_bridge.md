File_Code/rust-analyzer/ffca2c1fe4/syntax_bridge/syntax_bridge_after.rs --- 1/2 --- Rust
11     toknes: Vec<TextRange>,                                                                                                                               11     tokens: Vec<TextRange>,

File_Code/rust-analyzer/ffca2c1fe4/syntax_bridge/syntax_bridge_after.rs --- 2/2 --- Rust
35         self.toknes.get(idx).map(|&it| it)                                                                                                                35         self.tokens.get(idx).map(|&it| it)
36     }                                                                                                                                                     36     }
37                                                                                                                                                           37 
38     fn alloc(&mut self, relative_range: TextRange) -> tt::TokenId {                                                                                       38     fn alloc(&mut self, relative_range: TextRange) -> tt::TokenId {
39         let id = self.toknes.len();                                                                                                                       39         let id = self.tokens.len();
40         self.toknes.push(relative_range);                                                                                                                 40         self.tokens.push(relative_range);

