File_Code/nushell/a967854332/utils/utils_after.rs --- Rust
201     let mut stdout = std::io::stdout();                                                                                                                  201     let stdout = std::io::stdout();
202                                                                                                                                                          202 
203     if let PipelineData::RawStream(stream, _, _) = input {                                                                                               203     if let PipelineData::RawStream(stream, _, _) = input {
204         for s in stream {                                                                                                                                204         for s in stream {
205             let _ = stdout.write(s?.as_binary()?);                                                                                                       205             let _ = stdout.lock().write_all(s?.as_binary()?);

