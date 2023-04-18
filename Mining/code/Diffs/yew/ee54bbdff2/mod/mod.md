File_Code/yew/ee54bbdff2/mod/mod_after.rs --- Rust
  .                                                                                                                                                          100         match self {
100         write!(f, "{}", self)                                                                                                                            101             AttrValue::Static(s) => write!(f, "{}", s),
                                                                                                                                                             102             AttrValue::Owned(s) => write!(f, "{}", s),
                                                                                                                                                             103             AttrValue::Rc(s) => write!(f, "{}", s),
                                                                                                                                                             104         }

