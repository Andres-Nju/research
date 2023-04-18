File_Code/swc/082dc2101b/plugin/plugin_after.rs --- Rust
154                 r#"[package]                                                                                                                             154                 r#"[package]
155 name = "{}"                                                                                                                                              155 name = "{}"
156 version = "0.1.0"                                                                                                                                        156 version = "0.1.0"
157 edition = "2021"                                                                                                                                         157 edition = "2021"
158                                                                                                                                                          158 
159 [lib]                                                                                                                                                    159 [lib]
160 crate-type = ["cdylib"]                                                                                                                                  160 crate-type = ["cdylib"]
161                                                                                                                                                          161 
162 [profile.release]                                                                                                                                        162 [profile.release]
163 lto = true                                                                                                                                               163 lto = true
164                                                                                                                                                          164 
165 [dependencies]                                                                                                                                           165 [dependencies]
166 serde = "1"                                                                                                                                              166 serde = "1"
167 swc_core = {{ version = "{}", features = ["plugin_transform"] }}                                                                                         167 swc_core = {{ version = "{}", features = ["ecma_plugin_transform"] }}
168                                                                                                                                                          168 
169 # .cargo/config defines few alias to build plugin.                                                                                                       169 # .cargo/config defines few alias to build plugin.
170 # cargo build-wasi generates wasm-wasi32 binary                                                                                                          170 # cargo build-wasi generates wasm-wasi32 binary
171 # cargo build-wasm32 generates wasm32-unknown-unknown binary.                                                                                            171 # cargo build-wasm32 generates wasm32-unknown-unknown binary.
172 "#,                                                                                                                                                      172 "#,

