File_Code/cargo/f075f9c6cb/rename_deps/rename_deps_after.rs --- Rust
333             "\                                                                                                                                           333             "\
334 [DOCTEST] foo                                                                                                                                            334 [DOCTEST] foo
335 [RUNNING] `rustdoc --test [CWD]/src/lib.rs \                                                                                                             335 [RUNNING] `rustdoc --test [CWD]/src/lib.rs \
336         [..] \                                                                                                                                           336         [..] \
337         --extern baz=[CWD]/target/debug/deps/libbar-[..].rlib \                                                                                          337         --extern bar=[CWD]/target/debug/deps/libbar-[..].rlib \
338         --extern bar=[CWD]/target/debug/deps/libbar-[..].rlib \                                                                                          338         --extern baz=[CWD]/target/debug/deps/libbar-[..].rlib \
339         [..]`                                                                                                                                            339         [..]`
340 ",                                                                                                                                                       340 ",

