File_Code/rust/9dd2ee1942/intrinsics/intrinsics_after.rs --- Rust
851     /// // exact same size, and the same or lesser alignment, as the old                                                                                 851     /// // exact same size, and the same alignment, as the old type.
852     /// // type. The same caveats exist for this method as transmute, for                                                                                852     /// // The same caveats exist for this method as transmute, for
853     /// // the original inner type (`&i32`) to the converted inner type                                                                                  853     /// // the original inner type (`&i32`) to the converted inner type
854     /// // (`Option<&i32>`), so read the nomicon pages linked above.                                                                                     854     /// // (`Option<&i32>`), so read the nomicon pages linked above.
855     /// let v_from_raw = unsafe {                                                                                                                        855     /// let v_from_raw = unsafe {
856     ///     Vec::from_raw_parts(v_orig.as_mut_ptr(),                                                                                                     856     ///     Vec::from_raw_parts(v_orig.as_mut_ptr() as *mut Option<&i32>,

