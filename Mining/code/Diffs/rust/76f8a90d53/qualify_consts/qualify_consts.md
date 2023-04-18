File_Code/rust/76f8a90d53/qualify_consts/qualify_consts_after.rs --- 1/2 --- Text (8 errors, exceeded DFT_PARSE_ERROR_LIMIT)
  .                                                                                                                                                          738                             // in normal functions, mark such casts as not promotable
738                             self.add(Qualif::NOT_CONST);                                                                                                 739                             self.add(Qualif::NOT_CONST);
739                         } else if !self.tcx.sess.features_untracked().const_raw_ptr_to_usize_cast {                                                      740                         } else if !self.tcx.sess.features_untracked().const_raw_ptr_to_usize_cast {
                                                                                                                                                             741                             // in const fn and constants require the feature gate
                                                                                                                                                             742                             // FIXME: make it unsafe inside const fn and constants

File_Code/rust/76f8a90d53/qualify_consts/qualify_consts_after.rs --- 2/2 --- Text (8 errors, exceeded DFT_PARSE_ERROR_LIMIT)
...                                                                                                                                                          765                         // raw pointer operations are not allowed inside promoteds
762                         self.add(Qualif::NOT_CONST);                                                                                                     766                         self.add(Qualif::NOT_CONST);
763                     } else if !self.tcx.sess.features_untracked().const_compare_raw_pointers {                                                           767                     } else if !self.tcx.sess.features_untracked().const_compare_raw_pointers {
                                                                                                                                                             768                         // require the feature gate inside constants and const fn
                                                                                                                                                             769                         // FIXME: make it unsafe to use these operations

