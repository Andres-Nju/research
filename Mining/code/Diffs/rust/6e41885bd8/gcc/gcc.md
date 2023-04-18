File_Code/rust/6e41885bd8/gcc/gcc_after.rs --- Text (2 errors, exceeded DFT_PARSE_ERROR_LIMIT)
  .                                                                                                                                                          227         // Backtraces on ARM will call the personality routine with
  .                                                                                                                                                          228         // state == _US_VIRTUAL_UNWIND_FRAME | _US_FORCE_UNWIND. In those cases
  .                                                                                                                                                          229         // we want to continue unwinding the stack, otherwise all our backtraces
  .                                                                                                                                                          230         // would end at __rust_try.
227         if (state as c_int & uw::_US_ACTION_MASK as c_int)                                                                                               231         if (state as c_int & uw::_US_ACTION_MASK as c_int)
228                            == uw::_US_VIRTUAL_UNWIND_FRAME as c_int { // search phase                                                                    232                            == uw::_US_VIRTUAL_UNWIND_FRAME as c_int
                                                                                                                                                             233                && (state as c_int & uw::_US_FORCE_UNWIND as c_int) == 0 { // search phase

