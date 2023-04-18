        pub fn new() -> Self {
            unsafe {
                let mut set = std::mem::MaybeUninit::uninit();
                FD_ZERO(set.as_mut_ptr());
                Self {
                    set: set.assume_init(),
                }
            }
        }
