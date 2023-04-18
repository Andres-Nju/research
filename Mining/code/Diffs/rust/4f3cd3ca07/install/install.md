File_Code/rust/4f3cd3ca07/install/install_after.rs --- Rust
  .                                                                                                                                                          254             // Find the actual compiler (handling the full bootstrap option) which
  .                                                                                                                                                          255             // produced the save-analysis data because that data isn't copied
  .                                                                                                                                                          256             // through the sysroot uplifting.
254             compiler: self.compiler,                                                                                                                     257             compiler: builder.compiler_for(builder.top_stage, builder.config.build, self.target),

