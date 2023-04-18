File_Code/gfx/28e7e722fa/info/info_after.rs --- 1/2 --- Rust
340     let max_color_attachments = get_usize(gl, glow::MAX_COLOR_ATTACHMENTS).unwrap_or(8) as u8;                                                           340     let max_samples = get_usize(gl, glow::MAX_SAMPLES).unwrap_or(8);
                                                                                                                                                             341     let max_samples_mask = (max_samples * 2 - 1) as u8;

File_Code/gfx/28e7e722fa/info/info_after.rs --- 2/2 --- Rust
355         framebuffer_color_samples_count: max_color_attachments,                                                                                          356         framebuffer_color_samples_count: max_samples_mask,

