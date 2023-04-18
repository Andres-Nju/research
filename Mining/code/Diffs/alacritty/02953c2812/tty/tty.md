File_Code/alacritty/02953c2812/tty/tty_after.rs --- Rust
116         // Cross platform issue because on linux this is u64 as u64 (does nothing)                                                                       116         // TIOSCTTY changes based on platform and the `ioctl` call is different
117         // But on macos this is u32 as u64, asking for u64::from(u32)                                                                                    117         // based on architecture (32/64). So a generic cast is used to make sure
...                                                                                                                                                          118         // there are no issues. To allow such a generic cast the clippy warning
...                                                                                                                                                          119         // is disabled.
118         #[cfg_attr(feature = "clippy", allow(cast_lossless))]                                                                                            120         #[cfg_attr(feature = "clippy", allow(cast_lossless))]
119         libc::ioctl(fd, TIOCSCTTY as u64, 0)                                                                                                             121         libc::ioctl(fd, TIOCSCTTY as _, 0)

