File_Code/wasmer/bca702794c/memory/memory_after.rs --- 1/3 --- Rust
50         let protect = protect.to_protect_const();                                                                                                         50         let protect_const = protect.to_protect_const();

File_Code/wasmer/bca702794c/memory/memory_after.rs --- 2/3 --- Rust
72         let ptr = VirtualAlloc(start as _, size, MEM_COMMIT, protect);                                                                                    72         let ptr = VirtualAlloc(start as _, size, MEM_COMMIT, protect_const);

File_Code/wasmer/bca702794c/memory/memory_after.rs --- 3/3 --- Rust
77             self.protection = protection;                                                                                                                 77             self.protection = protect;

