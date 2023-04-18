File_Code/servo/d814c5b659/border/border_after.rs --- 1/2 --- Text (120 errors, exceeded DFT_PARSE_ERROR_LIMIT)
311             self.0.to_css(dest)                                                                                                                          311             self.1.to_css(dest)

File_Code/servo/d814c5b659/border/border_after.rs --- 2/2 --- Text (120 errors, exceeded DFT_PARSE_ERROR_LIMIT)
317             if self.1.is_some() {                                                                                                                        317             if let Some(second) = self.1 {
318                 try!(dest.write_str(" "));                                                                                                               318                 try!(dest.write_str(" "));
319                 try!(self.0.to_css(dest));                                                                                                               319                 try!(second.to_css(dest));

