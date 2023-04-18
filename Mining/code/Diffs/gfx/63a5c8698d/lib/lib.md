File_Code/gfx/63a5c8698d/lib/lib_after.rs --- 1/2 --- Rust
165                         CStr::from_ptr(inst_ext.extension_name.as_ptr()) ==                                                                              165                         CStr::from_ptr(inst_ext.extension_name.as_ptr()).to_bytes() == ext.as_bytes()
166                             CStr::from_ptr(ext.as_ptr() as *const _)                                                                                         

File_Code/gfx/63a5c8698d/lib/lib_after.rs --- 2/2 --- Rust
183                         CStr::from_ptr(inst_layer.layer_name.as_ptr()) ==                                                                                182                         CStr::from_ptr(inst_layer.layer_name.as_ptr()).to_bytes() == layer.as_bytes()
184                             CStr::from_ptr(layer.as_ptr() as *const _)                                                                                       

