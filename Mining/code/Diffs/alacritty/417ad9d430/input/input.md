File_Code/alacritty/417ad9d430/input/input_after.rs --- Rust
288             ctx.write_to_pty(contents.into_bytes());                                                                                                     288             ctx.write_to_pty(contents.replace("\x1b","").into_bytes());

