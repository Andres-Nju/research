File_Code/yew/d80e2f3e8a/vtag/vtag_after.rs --- 1/2 --- Rust
  .                                                                                                                                                          981         #[cfg(feature = "std_web")]
980         document().body().unwrap().append_child(&parent);                                                                                                982         document().body().unwrap().append_child(&parent);
                                                                                                                                                             983         #[cfg(feature = "web_sys")]
                                                                                                                                                             984         document().body().unwrap().append_child(&parent).unwrap();

File_Code/yew/d80e2f3e8a/vtag/vtag_after.rs --- 2/2 --- Rust
 ...                                                                                                                                                         1031         #[cfg(feature = "std_web")]
1026         document().body().unwrap().append_child(&parent);                                                                                               1032         document().body().unwrap().append_child(&parent);
                                                                                                                                                             1033         #[cfg(feature = "web_sys")]
                                                                                                                                                             1034         document().body().unwrap().append_child(&parent).unwrap();

