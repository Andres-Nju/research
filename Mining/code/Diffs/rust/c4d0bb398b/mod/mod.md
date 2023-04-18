File_Code/rust/c4d0bb398b/mod/mod_after.rs --- Rust
  .                                                                                                                                                          266                     // If the array is definitely non-empty, it's uninhabited if
  .                                                                                                                                                          267                     // the type of its elements is uninhabited.
266                     Some(n) if n != 0 => ty.uninhabited_from(visited, tcx),                                                                              268                     Some(n) if n != 0 => ty.uninhabited_from(visited, tcx),
267                     // If the array is definitely non-empty, it's uninhabited if                                                                             
268                     // the type of its elements is uninhabited.                                                                                              

