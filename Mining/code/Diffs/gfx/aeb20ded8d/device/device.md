File_Code/gfx/aeb20ded8d/device/device_after.rs --- Rust
1921     fn create_fence(&self, _signaled: bool) -> n::Fence {                                                                                               1919     fn create_fence(&self, signalled: bool) -> n::Fence {
1922         let mut handle = ptr::null_mut();                                                                                                               1920         let mut handle = ptr::null_mut();
1923         assert_eq!(winerror::S_OK, unsafe {                                                                                                             1921         assert_eq!(winerror::S_OK, unsafe {
1924             self.raw.clone().CreateFence(                                                                                                               1922             self.raw.clone().CreateFence(
1925                 0,                                                                                                                                      1923                 if signalled { 1 } else { 0 },

