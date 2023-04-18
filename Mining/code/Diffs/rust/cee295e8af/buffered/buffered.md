File_Code/rust/cee295e8af/buffered/buffered_after.rs --- 1/2 --- Rust
319 /// for i in 1..10 {                                                                                                                                     319 /// for i in 0..10 {
320 ///     stream.write(&[i]).unwrap();                                                                                                                     320 ///     stream.write(&[i+1]).unwrap();

File_Code/rust/cee295e8af/buffered/buffered_after.rs --- 2/2 --- Rust
335 /// for i in 1..10 {                                                                                                                                     335 /// for i in 0..10 {
336 ///     stream.write(&[i]).unwrap();                                                                                                                     336 ///     stream.write(&[i+1]).unwrap();

