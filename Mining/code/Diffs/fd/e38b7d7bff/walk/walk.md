File_Code/fd/e38b7d7bff/walk/walk_after.rs --- 1/3 --- Rust
164                     if entry.file_type().map_or(false, |ft| !ft.is_file()) {                                                                             164                     if entry.file_type().map_or(true, |ft| !ft.is_file()) {

File_Code/fd/e38b7d7bff/walk/walk_after.rs --- 2/3 --- Rust
169                     if entry.file_type().map_or(false, |ft| !ft.is_dir()) {                                                                              169                     if entry.file_type().map_or(true, |ft| !ft.is_dir()) {

File_Code/fd/e38b7d7bff/walk/walk_after.rs --- 3/3 --- Rust
174                     if entry.file_type().map_or(false, |ft| !ft.is_symlink()) {                                                                          174                     if entry.file_type().map_or(true, |ft| !ft.is_symlink()) {

