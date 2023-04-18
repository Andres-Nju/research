File_Code/solana/8c4f010b8d/snapshot_utils/snapshot_utils_after.rs --- Rust
   .                                                                                                                                                         1169     let remote_dir = build_snapshot_archives_remote_dir(snapshot_archives_dir);
   .                                                                                                                                                         1170     if remote_dir.exists() {
1169     ret.append(&mut walk_dir(                                                                                                                           1171         ret.append(&mut walk_dir(remote_dir.as_ref()));
1170         build_snapshot_archives_remote_dir(snapshot_archives_dir).as_ref(),                                                                             1172     }
1171     ));                                                                                                                                                      

