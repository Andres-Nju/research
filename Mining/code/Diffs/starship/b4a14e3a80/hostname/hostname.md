File_Code/starship/b4a14e3a80/hostname/hostname_after.rs --- 1/2 --- Rust
                                                                                                                                                            72     use unicode_segmentation::UnicodeSegmentation;

File_Code/starship/b4a14e3a80/hostname/hostname_after.rs --- 2/2 --- Rust
149         let (remainder, trim_at) = hostname.split_at(1);                                                                                                 150         let mut hostname_iter = hostname.graphemes(true);
                                                                                                                                                             151         let remainder = hostname_iter.next().unwrap_or_default();
                                                                                                                                                             152         let trim_at = hostname_iter.collect::<String>();

