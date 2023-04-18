File_Code/deno/fb358380c0/lib/lib_after.rs --- 1/2 --- Rust
                                                                                                                                                             6 use deno_core::error::generic_error;

File_Code/deno/fb358380c0/lib/lib_after.rs --- 2/2 --- Rust
436     .map_err(|_| deno_core::error::generic_error("Unable to build http client"))                                                                         437     .map_err(|e| generic_error(format!("Unable to build http client: {}", e)))

