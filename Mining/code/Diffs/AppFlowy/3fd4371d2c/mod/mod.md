File_Code/AppFlowy/3fd4371d2c/mod/mod_after.rs --- Rust
 .                                                                                                                                                           12 // Return the read me document content
12 pub fn initial_read_me() -> String {                                                                                                                      13 pub fn initial_read_me() -> String {
13   let document_content = include_str!("READ_ME.json");                                                                                                    14   let document_content = include_str!("READ_ME.json");
14   let transaction = make_transaction_from_document_content(document_content).unwrap();                                                                    15   return document_content.to_string();
15   transaction.to_json().unwrap()                                                                                                                             

