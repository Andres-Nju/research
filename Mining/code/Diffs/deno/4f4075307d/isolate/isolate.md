File_Code/deno/4f4075307d/isolate/isolate_after.rs --- 1/3 --- Rust
                                                                                                                                                           160     self.state.metrics_op_completed(buf.len() as u64);

File_Code/deno/4f4075307d/isolate/isolate_after.rs --- 2/3 --- Rust
288     isolate.state.metrics_op_completed(buf_size as u64);                                                                                                     

File_Code/deno/4f4075307d/isolate/isolate_after.rs --- 3/3 --- Rust
300         let buf_size = buf.len();                                                                                                                        ... 
301         state.send_to_js(req_id, buf);                                                                                                                   299         state.send_to_js(req_id, buf);
302         state.metrics_op_completed(buf_size as u64);                                                                                                         

