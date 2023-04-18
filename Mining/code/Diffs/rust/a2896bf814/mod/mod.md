File_Code/rust/a2896bf814/mod/mod_after.rs --- Rust
247             concat!("Converts a string slice in a given base to an integer.                                                                              247             concat!("Converts a string slice in a given base to an integer.
248                                                                                                                                                          248 
249 The string is expected to be an optional `+` or `-` sign followed by digits.                                                                             249 The string is expected to be an optional `+` or `-` sign followed by digits.
250 Leading and trailing whitespace represent an error. Digits are a subset of these characters,                                                             250 Leading and trailing whitespace represent an error. Digits are a subset of these characters,
251 depending on `radix`:                                                                                                                                    251 depending on `radix`:
252                                                                                                                                                          252 
253  * `0-9`                                                                                                                                                 253  * `0-9`
254  * `a-z`                                                                                                                                                 254  * `a-z`
255  * `a-z`                                                                                                                                                 255  * `A-Z`
256                                                                                                                                                          256 
257 # Panics                                                                                                                                                 257 # Panics
258                                                                                                                                                          258 
259 This function panics if `radix` is not in the range from 2 to 36.                                                                                        259 This function panics if `radix` is not in the range from 2 to 36.
260                                                                                                                                                          260 
261 # Examples                                                                                                                                               261 # Examples
262                                                                                                                                                          262 
263 Basic usage:                                                                                                                                             263 Basic usage:
264                                                                                                                                                          264 
265 ```                                                                                                                                                      265 ```
266 ", $Feature, "assert_eq!(", stringify!($SelfT), "::from_str_radix(\"A\", 16), Ok(10));",                                                                 266 ", $Feature, "assert_eq!(", stringify!($SelfT), "::from_str_radix(\"A\", 16), Ok(10));",

