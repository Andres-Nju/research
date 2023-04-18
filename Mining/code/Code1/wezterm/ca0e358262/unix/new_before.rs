        pub fn new() -> Self {
            unsafe {
                let mut set = std::mem::MaybeUninit::uninit().assume_init();
                FD_ZERO(&mut set);
                Self { set }
            }
        }
