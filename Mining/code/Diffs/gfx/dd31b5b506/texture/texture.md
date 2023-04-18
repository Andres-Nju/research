File_Code/gfx/dd31b5b506/texture/texture_after.rs --- 1/2 --- Rust
417             (u<<8) + (c * 255.0 + 0.5) as u32                                                                                                            417             (u<<8) + (c * 255.0) as u32

File_Code/gfx/dd31b5b506/texture/texture_after.rs --- 2/2 --- Rust
427             out[i] = (byte as f32 + 0.5) / 255.0;                                                                                                        427             out[i] = byte as f32 / 255.0;

