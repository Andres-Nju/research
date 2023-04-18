File_Code/rust/357c5dacee/vec/vec_after.rs --- 1/2 --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
2413                     // Read from a properly aligned pointer to make up a value of this ZST.                                                             2413                     // Make up a value of this ZST.
2414                     Some(ptr::read(NonNull::dangling().as_ptr()))                                                                                       2414                     Some(mem::zeroed())

File_Code/rust/357c5dacee/vec/vec_after.rs --- 2/2 --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
2453                     // Read from a properly aligned pointer to make up a value of this ZST.                                                             2453                     // Make up a value of this ZST.
2454                     Some(ptr::read(NonNull::dangling().as_ptr()))                                                                                       2454                     Some(mem::zeroed())

