File_Code/swc/b46815a684/mod/mod_after.rs --- Text (8 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1312             FileName::Real(path) if !paths.is_empty() && !path.is_absolute() => {                                                                       1312             FileName::Real(path) if !paths.is_empty() && !path.is_absolute() => FileName::Real(
1313                 FileName::Real(std::env::current_dir().unwrap().join(path))                                                                             1313                 std::env::current_dir()
....                                                                                                                                                         1314                     .map(|v| v.join(path))
1314             }                                                                                                                                           1315                     .unwrap_or_else(|_| path.to_path_buf()),
                                                                                                                                                             1316             ),

