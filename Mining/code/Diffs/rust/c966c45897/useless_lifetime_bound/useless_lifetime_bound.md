File_Code/rust/c966c45897/useless_lifetime_bound/useless_lifetime_bound_after.rs --- Rust
                                                                                                                                                             9 // @has useless_lifetime_bound/struct.Scope.html
                                                                                                                                                            10 // @!has - '//*[@class="rust struct"]' "T: 'a + 'a"
                                                                                                                                                            11 pub struct SomeStruct<'a, T: 'a> {
                                                                                                                                                            12     _marker: PhantomData<&'a T>,
                                                                                                                                                            13 }

