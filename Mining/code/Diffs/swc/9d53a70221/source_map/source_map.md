File_Code/swc/9d53a70221/source_map/source_map_after.rs --- Rust
1064             let chpos = { self.calc_extra_bytes(&f, &mut ch_start, pos) };                                                                              1064             let chpos = pos.to_u32() - self.calc_extra_bytes(&f, &mut ch_start, pos);
1065             let linechpos = { self.calc_extra_bytes(&f, &mut line_ch_start, linebpos) };                                                                1065             let linechpos =
                                                                                                                                                             1066                 linebpos.to_u32() - self.calc_extra_bytes(&f, &mut line_ch_start, linebpos);

