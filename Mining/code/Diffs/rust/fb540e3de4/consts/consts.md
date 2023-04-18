File_Code/rust/fb540e3de4/consts/consts_after.rs --- Rust
339                 // References to a static are inherently promotable,                                                                                     339                 // References to a static that are themselves within a static
340                 // with the exception of "#[thread_loca]" statics.                                                                                       340                 // are inherently promotable with the exception
...                                                                                                                                                          341                 //  of "#[thread_loca]" statics, which may not
341                 // The latter may not outlive the current function                                                                                       342                 // outlive the current function

