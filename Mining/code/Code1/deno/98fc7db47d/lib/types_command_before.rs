fn types_command() {
  println!(
    "{}\n{}\n{}",
    crate::js::DENO_NS_LIB,
    crate::js::SHARED_GLOBALS_LIB,
    crate::js::WINDOW_LIB
  );
}
