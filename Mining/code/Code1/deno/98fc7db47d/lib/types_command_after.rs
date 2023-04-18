fn types_command() {
  let types = format!(
    "{}\n{}\n{}",
    crate::js::DENO_NS_LIB,
    crate::js::SHARED_GLOBALS_LIB,
    crate::js::WINDOW_LIB
  );
  use std::io::Write;
  let _r = std::io::stdout().write_all(types.as_bytes());
  // TODO(ry) Only ignore SIGPIPE. Currently ignoring all errors.
}
