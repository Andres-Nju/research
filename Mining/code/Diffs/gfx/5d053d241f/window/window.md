File_Code/gfx/5d053d241f/window/window_after.rs --- Rust
568         timeout_ns -= moment.elapsed().as_nanos() as u64;                                                                                                568         timeout_ns = timeout_ns.saturating_sub(moment.elapsed().as_nanos() as u64);

