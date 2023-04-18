File_Code/nushell/ccebdd7a7f/repl/repl_after.rs --- 1/3 --- Rust
790                         let cwd = get_guaranteed_cwd(engine_state, stack);                                                                                 
791                         engine_state.merge_env(stack, cwd)?;                                                                                               

File_Code/nushell/ccebdd7a7f/repl/repl_after.rs --- 2/3 --- Rust
799                         let cwd = get_guaranteed_cwd(engine_state, stack);                                                                                 
800                         engine_state.merge_env(stack, cwd)?;                                                                                               

File_Code/nushell/ccebdd7a7f/repl/repl_after.rs --- 3/3 --- Rust
                                                                                                                                                             823     let cwd = get_guaranteed_cwd(engine_state, stack);
                                                                                                                                                             824     engine_state.merge_env(stack, cwd)?;

