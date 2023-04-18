File_Code/nushell/8f5b857fcf/ignore/ignore_after.rs --- Rust
26         _input: PipelineData,                                                                                                                             26         input: PipelineData,
27     ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {                                                                                     27     ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
                                                                                                                                                             28         input.into_value(call.head);

