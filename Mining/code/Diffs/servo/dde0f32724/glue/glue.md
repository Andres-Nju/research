File_Code/servo/dde0f32724/glue/glue_after.rs --- 1/2 --- Rust
                                                                                                                                                          1554     use std::mem;

File_Code/servo/dde0f32724/glue/glue_after.rs --- 2/2 --- Rust
                                                                                                                                                          1618                     // We only make sure we have enough space for this variable,
                                                                                                                                                          1619                     // but didn't construct a default value for StyleAnimationValue,
                                                                                                                                                          1620                     // so we should zero it to avoid getting undefined behaviors.
                                                                                                                                                          1621                     animation_values[i].mValue.mGecko = unsafe { mem::zeroed() };

