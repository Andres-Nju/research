File_Code/deno/18150b3a78/main/main_after.rs --- Rust
343     let clone = strace_result.get("clone").map(|d| d.calls).unwrap_or(0);                                                                                343     let clone = strace_result.get("clone").map(|d| d.calls).unwrap_or(0) + 1;

