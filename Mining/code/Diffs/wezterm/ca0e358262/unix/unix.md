File_Code/wezterm/ca0e358262/unix/unix_after.rs --- Rust
464                 let mut set = std::mem::MaybeUninit::uninit().assume_init();                                                                             464                 let mut set = std::mem::MaybeUninit::uninit();
465                 FD_ZERO(&mut set);                                                                                                                       465                 FD_ZERO(set.as_mut_ptr());
466                 Self { set }                                                                                                                             466                 Self {
                                                                                                                                                             467                     set: set.assume_init(),

