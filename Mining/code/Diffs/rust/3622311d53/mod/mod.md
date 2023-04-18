File_Code/rust/3622311d53/mod/mod_after.rs --- 1/2 --- Rust
12 use crate::session::config::{OutputType, SwitchWithOptPath};                                                                                              12 use crate::session::config::{OutputType, PrintRequest, SwitchWithOptPath};

File_Code/rust/3622311d53/mod/mod_after.rs --- 2/2 --- Rust
  ..                                                                                                                                                         1309     // We should only display this error if we're actually going to run PGO.
  ..                                                                                                                                                         1310     // If we're just supposed to print out some data, don't show the error (#61002).
1309     if sess.opts.cg.profile_generate.enabled() &&                                                                                                       1311     if sess.opts.cg.profile_generate.enabled() &&
1310        sess.target.target.options.is_like_msvc &&                                                                                                       1312        sess.target.target.options.is_like_msvc &&
1311        sess.panic_strategy() == PanicStrategy::Unwind {                                                                                                 1313        sess.panic_strategy() == PanicStrategy::Unwind &&
                                                                                                                                                             1314        sess.opts.prints.iter().all(|&p| p == PrintRequest::NativeStaticLibs) {

