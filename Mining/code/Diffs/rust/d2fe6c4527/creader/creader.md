File_Code/rust/d2fe6c4527/creader/creader_after.rs --- Rust
989                     self.sess.err("no #[default_lib_allocator] found but one is \                                                                        989                     self.sess.err("no global memory allocator found but one is \
...                                                                                                                                                          990                                    required; link to std or \
...                                                                                                                                                          991                                    add #[global_allocator] to a static item \
990                                    required; is libstd not linked?");                                                                                    992                                    that implements the GlobalAlloc trait.");

