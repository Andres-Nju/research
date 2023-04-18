File_Code/rust/890d04d00f/mod/mod_after.rs --- Rust
23 //                                                                                                                                                         . 
24 // Since slices don't support inherent methods; all operations                                                                                             . 
25 // on them are defined on traits, which are then re-exported from                                                                                          . 
26 // the prelude for convenience. So there are a lot of traits here.                                                                                         . 
27 //                                                                                                                                                        23 //
28 // The layout of this file is thus:                                                                                                                       24 // The layout of this file is thus:
29 //                                                                                                                                                        25 //
30 // * Slice-specific 'extension' traits and their implementations. This                                                                                    .. 
31 //   is where most of the slice API resides.                                                                                                              26 // * Inherent methods. This is where most of the slice API resides.

