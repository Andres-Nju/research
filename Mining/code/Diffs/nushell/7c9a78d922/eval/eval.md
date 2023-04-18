File_Code/nushell/7c9a78d922/eval/eval_after.rs --- Rust
                                                                                                                                                            32     if let Some(ctrlc) = &engine_state.ctrlc {
                                                                                                                                                            33         if ctrlc.load(core::sync::atomic::Ordering::SeqCst) {
                                                                                                                                                            34             return Ok(Value::Nothing { span: call.head }.into_pipeline_data());
                                                                                                                                                            35         }
                                                                                                                                                            36     }

