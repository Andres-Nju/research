File_Code/gfx/683f2fe9f7/device/device_after.rs --- 1/2 --- Rust
377     fn destroy_buffer(&self, B::Buffer);                                                                                                                 377     fn destroy_buffer(&self, buffer: B::Buffer);

File_Code/gfx/683f2fe9f7/device/device_after.rs --- 2/2 --- Rust
414         &B::Memory,                                                                                                                                      414         memory: &B::Memory,
415         offset: u64,                                                                                                                                     415         offset: u64,
416         B::UnboundImage,                                                                                                                                 416         image: B::UnboundImage,

