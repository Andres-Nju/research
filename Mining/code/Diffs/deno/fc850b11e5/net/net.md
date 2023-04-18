File_Code/deno/fc850b11e5/net/net_after.rs --- Rust
247         socket                                                                                                                                           247         let byte_length = socket
248           .send_to(&zero_copy, &resource.local_addr.as_pathname().unwrap())                                                                              248           .send_to(&zero_copy, &resource.local_addr.as_pathname().unwrap())
249           .await?;                                                                                                                                       249           .await?;
250                                                                                                                                                          250 
251         Ok(json!({}))                                                                                                                                    251         Ok(json!(byte_length))

