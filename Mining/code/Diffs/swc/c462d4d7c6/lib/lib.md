File_Code/swc/c462d4d7c6/lib/lib_after.rs --- Rust
 .                                                                                                                                                           20         .with_writer(std::io::stderr)
20         .with_ansi(true)                                                                                                                                  21         .with_ansi(true)
21         .with_env_filter(EnvFilter::from_env("SWC_LOG"))                                                                                                  22         .with_env_filter(EnvFilter::from_env("SWC_LOG"))
22         .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::ERROR.into()))                                                       23         .pretty()

