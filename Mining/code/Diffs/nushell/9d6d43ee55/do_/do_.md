File_Code/nushell/9d6d43ee55/do_/do__after.rs --- Rust
221             Ok(PipelineData::Value(..)) | Err(_) if ignore_shell_errors => {                                                                             221             Ok(PipelineData::Value(Value::Error { .. }, ..)) | Err(_) if ignore_shell_errors => {

