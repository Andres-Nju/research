File_Code/rust-analyzer/3c68da792b/parser/parser_after.rs --- 1/2 --- Text (122 errors, exceeded DFT_PARSE_ERROR_LIMIT)
8     SyntaxKind::{self, EOF, ERROR, L_DOLLAR, R_DOLLAR, TOMBSTONE},                                                                                         8     SyntaxKind::{self, EOF, ERROR, TOMBSTONE},

File_Code/rust-analyzer/3c68da792b/parser/parser_after.rs --- 2/2 --- Text (122 errors, exceeded DFT_PARSE_ERROR_LIMIT)
215             T!['{'] | T!['}'] | L_DOLLAR | R_DOLLAR => {                                                                                                 215             T!['{'] | T!['}'] => {

