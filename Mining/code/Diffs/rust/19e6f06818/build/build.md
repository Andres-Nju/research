File_Code/rust/19e6f06818/build/build_after.rs --- 1/3 --- Rust
99                       -> P<ast::Stmt>;                                                                                                                    99                       -> ast::Stmt;

File_Code/rust/19e6f06818/build/build_after.rs --- 2/3 --- Rust
559                       -> P<ast::Stmt> {                                                                                                                  559                       -> ast::Stmt {

File_Code/rust/19e6f06818/build/build_after.rs --- 3/3 --- Rust
574         P(ast::Stmt {                                                                                                                                    574         ast::Stmt {
575             id: ast::DUMMY_NODE_ID,                                                                                                                      575             id: ast::DUMMY_NODE_ID,
576             node: ast::StmtKind::Local(local),                                                                                                           576             node: ast::StmtKind::Local(local),
577             span: sp,                                                                                                                                    577             span: sp,
578         })                                                                                                                                               578         }

