File_Code/nushell/28947ff9a9/griddle/griddle_after.rs --- 1/4 --- Rust
34                 "number of columns wide",                                                                                                                 34                 "number of terminal columns wide (not output columns)",

File_Code/nushell/28947ff9a9/griddle/griddle_after.rs --- 2/4 --- Rust
63         let width_param: Option<String> = call.get_flag(engine_state, stack, "width")?;                                                                   63         let width_param: Option<i64> = call.get_flag(engine_state, stack, "width")?;

File_Code/nushell/28947ff9a9/griddle/griddle_after.rs --- 3/4 --- Rust
159     width_param: Option<String>,                                                                                                                         159     width_param: Option<i64>,

File_Code/nushell/28947ff9a9/griddle/griddle_after.rs --- 4/4 --- Rust
171         col.parse::<u16>().unwrap_or(80)                                                                                                                 171         col as u16

