File_Code/rust/262029eca9/dist/dist_after.rs --- 1/4 --- Rust
1162         // We expect RLS to build, because we've exited this step above if tool                                                                           
1163         // state for RLS isn't testing.                                                                                                                   

File_Code/rust/262029eca9/dist/dist_after.rs --- 2/4 --- Rust
                                                                                                                                                             1263         let rustfmt_installer = builder.ensure(Rustfmt { stage, target });

File_Code/rust/262029eca9/dist/dist_after.rs --- 3/4 --- Rust
                                                                                                                                                             1301         tarballs.extend(rustfmt_installer.clone());

File_Code/rust/262029eca9/dist/dist_after.rs --- 4/4 --- Rust
                                                                                                                                                             1369             if rustfmt_installer.is_none() {
                                                                                                                                                             1370                 contents = filter(&contents, "rustfmt");
                                                                                                                                                             1371             }

