File_Code/rust/4e1249d75f/mod/mod_after.rs --- Text (6 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1212                 let region = {                                                                                                                          1212                 let region = if ppaux::verbose() || ppaux::identify_regions() {
1213                     let mut region = format!("{}", region);                                                                                             1213                     let mut region = format!("{}", region);
1214                     if region.len() > 0 { region.push(' '); }                                                                                           1214                     if region.len() > 0 { region.push(' '); }
1215                     region                                                                                                                              1215                     region
                                                                                                                                                             1216                 } else {
                                                                                                                                                             1217                     // Do not even print 'static
                                                                                                                                                             1218                     "".to_owned()

