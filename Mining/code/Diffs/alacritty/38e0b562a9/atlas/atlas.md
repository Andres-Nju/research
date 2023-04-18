File_Code/alacritty/38e0b562a9/atlas/atlas_after.rs --- Rust
  .                                                                                                                                                          262                 // Get the context type before adding a new Atlas.
  .                                                                                                                                                          263                 let is_gles_context = atlas[*current_atlas].is_gles_context;
  .                                                                                                                                                          264 
  .                                                                                                                                                          265                 // Advance the current Atlas index.
262                 *current_atlas += 1;                                                                                                                     266                 *current_atlas += 1;
263                 if *current_atlas == atlas.len() {                                                                                                       267                 if *current_atlas == atlas.len() {
264                     let new = Atlas::new(ATLAS_SIZE, atlas[*current_atlas].is_gles_context);                                                             268                     let new = Atlas::new(ATLAS_SIZE, is_gles_context);

