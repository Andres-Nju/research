File_Code/alacritty/e2e25b3206/event/event_after.rs --- 1/2 --- Rust
17 use crate::config::{self, Config};                                                                                                                        17 use crate::config::{self, Config, StartupMode};

File_Code/alacritty/e2e25b3206/event/event_after.rs --- 2/2 --- Rust
345             is_fullscreen: false,                                                                                                                        345             is_fullscreen: config.window.startup_mode() == StartupMode::Fullscreen,
                                                                                                                                                             346             #[cfg(target_os = "macos")]
                                                                                                                                                             347             is_simple_fullscreen: config.window.startup_mode() == StartupMode::SimpleFullscreen,
                                                                                                                                                             348             #[cfg(not(target_os = "macos"))]

