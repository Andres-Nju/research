File_Code/rust/6aa5a5df96/render/render_after.rs --- Text (22 errors, exceeded DFT_PARSE_ERROR_LIMIT)
1322                             // This closure prevents crates' name to be aggregated. It allows to not                                                    1322                             // This closure prevents crates' names from being aggregated.
....                                                                                                                                                         1323                             //
1323                             // have to look for crate's name into the strings array.                                                                    1324                             // The point here is to check if the string is preceded by '[' and
                                                                                                                                                             1325                             // "searchIndex". If so, it means this is a crate name and that it
                                                                                                                                                             1326                             // shouldn't be aggregated.

