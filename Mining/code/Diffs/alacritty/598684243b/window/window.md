File_Code/alacritty/598684243b/window/window_after.rs --- 1/4 --- Rust
7     glutin::platform::unix::{EventLoopWindowTargetExtUnix, WindowBuilderExtUnix, WindowExtUnix},                                                           7     glutin::platform::unix::{WindowBuilderExtUnix, WindowExtUnix},

File_Code/alacritty/598684243b/window/window_after.rs --- 2/4 --- Rust
                                                                                                                                                            16     glutin::platform::unix::EventLoopWindowTargetExtUnix,

File_Code/alacritty/598684243b/window/window_after.rs --- 3/4 --- Rust
342     #[cfg(any(not(feature = "x11"), windows))]                                                                                                           343     #[cfg(any(windows, not(any(feature = "x11", target_os = "macos"))))]

File_Code/alacritty/598684243b/window/window_after.rs --- 4/4 --- Rust
400     #[cfg(any(not(feature = "wayland"), any(target_os = "macos", windows)))]                                                                             401     #[cfg(not(any(feature = "wayland", target_os = "macos", windows)))]

