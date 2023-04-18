pub fn not_supported() -> AnyError {
  custom_error("NotSupported", "The operation is not supported")
}
