File_Code/servo/37879260a9/glue/glue_after.rs --- Rust
                                                                                                                                                           185 #[no_mangle]
                                                                                                                                                           186 pub extern "C" fn Servo_InitializeCooperativeThread() {
                                                                                                                                                           187     // Pretend that we're a Servo Layout thread to make some assertions happy.
                                                                                                                                                           188     thread_state::initialize(thread_state::LAYOUT);
                                                                                                                                                           189 }

