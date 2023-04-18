File_Code/swc/38fb70c237/plugin/plugin_after.rs --- Rust
205         let dist_output_path = format!("target/{}/release/{}.wasm", build_target, name);                                                                 205         let dist_output_path = format!("target/{}/release/{}.wasm", build_target, name.replace("-", "_"));

