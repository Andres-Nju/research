File_Code/rust/f633284b3d/dladdr/dladdr_after.rs --- Rust
25         let symname = if dladdr(frame.exact_position, &mut info) == 0 {                                                                                   25         let symname = if dladdr(frame.exact_position, &mut info) == 0 ||
                                                                                                                                                             26                          info.dli_sname.is_null() {

