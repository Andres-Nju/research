File_Code/rust/cc0ab82091/suggest/suggest_after.rs --- 1/2 --- Rust
268                                         let is_real_filename = match filename {                                                                            
269                                             FileName::Real(_) => true,                                                                                     
270                                             _ => false,                                                                                                    
271                                         };                                                                                                                 

File_Code/rust/cc0ab82091/suggest/suggest_after.rs --- 2/2 --- Rust
281                                         match (is_real_filename, parent_node) {                                                                          277                                         match (filename, parent_node) {
282                                             (true, hir_map::NodeLocal(hir::Local {                                                                       278                                             (FileName::Real(_), hir_map::NodeLocal(hir::Local {

