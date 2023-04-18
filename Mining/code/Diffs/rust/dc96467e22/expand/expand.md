File_Code/rust/dc96467e22/expand/expand_after.rs --- 1/2 --- Rust
                                                                                                                                                           488                                .map_err(|mut e| { e.emit(); }).ok()?;

File_Code/rust/dc96467e22/expand/expand_after.rs --- 2/2 --- Rust
493                 let meta = attr.parse_meta(self.cx.parse_sess).ok()?;                                                                                    494                 let meta = attr.parse_meta(self.cx.parse_sess)
                                                                                                                                                             495                                .expect("derive meta should already have been parsed");

