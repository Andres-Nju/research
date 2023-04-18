File_Code/rust/8ed83f4aaf/fast_thread_local/fast_thread_local_after.rs --- 1/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
60 unsafe fn register_dtor(t: *mut u8, dtor: unsafe extern fn(*mut u8)) {                                                                                    60 pub unsafe fn register_dtor(t: *mut u8, dtor: unsafe extern fn(*mut u8)) {

File_Code/rust/8ed83f4aaf/fast_thread_local/fast_thread_local_after.rs --- 2/2 --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                            117 }
                                                                                                                                                            118 
                                                                                                                                                            119 pub fn requires_move_before_drop() -> bool {
                                                                                                                                                            120     false

