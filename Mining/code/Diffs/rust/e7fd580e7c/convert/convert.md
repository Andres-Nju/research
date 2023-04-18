File_Code/rust/e7fd580e7c/convert/convert_after.rs --- 1/2 --- Rust
441 /// struct SuperiorThanZero(i32);                                                                                                                        441 /// struct GreaterThanZero(i32);
442 ///                                                                                                                                                      442 ///
443 /// impl TryFrom<i32> for SuperiorThanZero {                                                                                                             443 /// impl TryFrom<i32> for GreaterThanZero {

File_Code/rust/e7fd580e7c/convert/convert_after.rs --- 2/2 --- Rust
448 ///             Err("SuperiorThanZero only accepts value superior than zero!")                                                                           448 ///             Err("GreaterThanZero only accepts value superior than zero!")
449 ///         } else {                                                                                                                                     449 ///         } else {
450 ///             Ok(SuperiorThanZero(value))                                                                                                              450 ///             Ok(GreaterThanZero(value))

