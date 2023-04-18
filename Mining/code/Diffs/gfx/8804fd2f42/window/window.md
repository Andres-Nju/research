File_Code/gfx/8804fd2f42/window/window_after.rs --- Rust
257             let (width, height) = window.get_inner_size().unwrap();                                                                                      257             let logical_size = window.get_inner_size().unwrap();
...                                                                                                                                                          258             let width = logical_size.width * window.get_hidpi_factor();
...                                                                                                                                                          259             let height = logical_size.height * window.get_hidpi_factor();
258             self.create_surface_android(window.get_native_window(), width, height)                                                                       260             self.create_surface_android(window.get_native_window(), width as _, height as _)

