File_Code/gfx/e504226c19/main/main_after.rs --- 1/3 --- Rust
88         buffer::Usage::TRANSFER_SRC,                                                                                                                      88         buffer::Usage::TRANSFER_SRC | buffer::Usage::TRANSFER_DST,

File_Code/gfx/e504226c19/main/main_after.rs --- 2/3 --- Rust
103         buffer::Usage::TRANSFER_DST,                                                                                                                     103         buffer::Usage::TRANSFER_SRC | buffer::Usage::TRANSFER_DST | buffer::Usage::STORAGE,

File_Code/gfx/e504226c19/main/main_after.rs --- 3/3 --- Rust
                                                                                                                                                             156     device.destroy_command_pool(command_pool.downgrade());

