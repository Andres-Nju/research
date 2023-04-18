File_Code/gfx/8e8d8016b3/main/main_after.rs --- 1/3 --- Rust
                                                                                                                                                           124         // Return `window` so it is not dropped: dropping it invalidates `surface`.

File_Code/gfx/8e8d8016b3/main/main_after.rs --- 2/3 --- Rust
622                     .expect("Could not create semaphore"),                                                                                               623                     .expect("Could not create fence"),

File_Code/gfx/8e8d8016b3/main/main_after.rs --- 3/3 --- Rust
                                                                                                                                                             931             self.surface
                                                                                                                                                             932               .unconfigure_swapchain(&self.device);

