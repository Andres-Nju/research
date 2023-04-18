File_Code/rust/999c2e2433/range/range_after.rs --- Rust
169 #[cfg(not(target_pointer_witdth = "16"))]                                                                                                                169 #[cfg(not(target_pointer_width = "16"))]
170 step_impl_unsigned!(u32);                                                                                                                                170 step_impl_unsigned!(u32);
171 #[cfg(target_pointer_witdth = "16")]                                                                                                                     171 #[cfg(target_pointer_width = "16")]
172 step_impl_no_between!(u32);                                                                                                                              172 step_impl_no_between!(u32);
173 step_impl_signed!([isize: usize] [i8: u8] [i16: u16]);                                                                                                   173 step_impl_signed!([isize: usize] [i8: u8] [i16: u16]);
174 #[cfg(not(target_pointer_witdth = "16"))]                                                                                                                174 #[cfg(not(target_pointer_width = "16"))]
175 step_impl_signed!([i32: u32]);                                                                                                                           175 step_impl_signed!([i32: u32]);
176 #[cfg(target_pointer_witdth = "16")]                                                                                                                     176 #[cfg(target_pointer_width = "16")]

