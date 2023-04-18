File_Code/deno/fb358380c0/http_util/http_util_after.rs --- Rust
37     .map_err(|_| generic_error("Unable to build http client"))                                                                                            37     .map_err(|e| generic_error(format!("Unable to build http client: {}", e)))

