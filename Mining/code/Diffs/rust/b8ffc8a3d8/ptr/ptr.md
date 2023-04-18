File_Code/rust/b8ffc8a3d8/ptr/ptr_after.rs --- Text (4 errors, exceeded DFT_PARSE_ERROR_LIMIT)
                                                                                                                                                          2557     /// Cast to a pointer of another type
                                                                                                                                                          2558     #[unstable(feature = "nonnull_cast", issue = "47653")]
                                                                                                                                                          2559     pub fn cast<U>(self) -> NonNull<U> {
                                                                                                                                                          2560         unsafe {
                                                                                                                                                          2561             NonNull::new_unchecked(self.as_ptr() as *mut U)
                                                                                                                                                          2562         }
                                                                                                                                                          2563     }

