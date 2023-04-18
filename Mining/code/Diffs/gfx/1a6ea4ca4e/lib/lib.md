File_Code/gfx/1a6ea4ca4e/lib/lib_after.rs --- Rust
604             if winerror::SUCCEEDED(hr) {                                                                                                                 604             if winerror::SUCCEEDED(hr) {
605                 unsafe { (*debug_controller).EnableDebugLayer() };                                                                                       605                 unsafe { (*debug_controller).EnableDebugLayer() };
606             }                                                                                                                                            ... 
607                                                                                                                                                          ... 
608             unsafe { (*debug_controller).Release(); }                                                                                                    606                 unsafe { (*debug_controller).Release(); }
                                                                                                                                                             607             }

