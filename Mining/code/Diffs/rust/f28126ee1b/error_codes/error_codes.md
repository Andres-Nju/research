File_Code/rust/f28126ee1b/error_codes/error_codes_after.rs --- Rust
337 E0080: r##"                                                                                                                                              337 E0080: r##"
338 This error indicates that the compiler was unable to sensibly evaluate an                                                                                338 This error indicates that the compiler was unable to sensibly evaluate a
339 constant expression that had to be evaluated. Attempting to divide by 0                                                                                  339 constant expression that had to be evaluated. Attempting to divide by 0
340 or causing integer overflow are two ways to induce this error. For example:                                                                              340 or causing integer overflow are two ways to induce this error. For example:
341                                                                                                                                                          341 
342 ```compile_fail,E0080                                                                                                                                    342 ```compile_fail,E0080
343 enum Enum {                                                                                                                                              343 enum Enum {
344     X = (1 << 500),                                                                                                                                      344     X = (1 << 500),
345     Y = (1 / 0)                                                                                                                                          345     Y = (1 / 0)
346 }                                                                                                                                                        346 }
347 ```                                                                                                                                                      347 ```
348                                                                                                                                                          348 
349 Ensure that the expressions given can be evaluated as the desired integer type.                                                                          349 Ensure that the expressions given can be evaluated as the desired integer type.
350 See the FFI section of the Reference for more information about using a custom                                                                           350 See the FFI section of the Reference for more information about using a custom
351 integer type:                                                                                                                                            351 integer type:
352                                                                                                                                                          352 
353 https://doc.rust-lang.org/reference.html#ffi-attributes                                                                                                  353 https://doc.rust-lang.org/reference.html#ffi-attributes
354 "##,                                                                                                                                                     354 "##,

