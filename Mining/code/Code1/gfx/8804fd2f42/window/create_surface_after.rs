    pub fn create_surface(&self, window: &winit::Window) -> Surface {
        #[cfg(all(unix, not(target_os = "android")))]
        {
            use winit::os::unix::WindowExt;

            if self.extensions.contains(&vk::VK_KHR_WAYLAND_SURFACE_EXTENSION_NAME) {
                if let Some(display) = window.get_wayland_display() {
                    let display: *mut c_void = display as *mut _;
                    let surface: *mut c_void = window.get_wayland_surface().unwrap() as *mut _;
                    let px = window.get_inner_size().unwrap();
                    return self.create_surface_from_wayland(display, surface, px.width as _, px.height as _);
                }
            }
            if self.extensions.contains(&vk::VK_KHR_XLIB_SURFACE_EXTENSION_NAME) {
                if let Some(display) = window.get_xlib_display() {
                    let window = window.get_xlib_window().unwrap();
                    return self.create_surface_from_xlib(display as _, window);
                }
            }
            panic!("The Vulkan driver does not support surface creation!");
        }
        #[cfg(target_os = "android")]
        {
            use winit::os::android::WindowExt;
            let logical_size = window.get_inner_size().unwrap();
            let width = logical_size.width * window.get_hidpi_factor();
            let height = logical_size.height * window.get_hidpi_factor();
            self.create_surface_android(window.get_native_window(), width as _, height as _)
        }
        #[cfg(windows)]
        {
            use winapi::um::libloaderapi::GetModuleHandleW;
            use winit::os::windows::WindowExt;

            let hinstance = unsafe { GetModuleHandleW(ptr::null()) };
            let hwnd = window.get_hwnd();
            self.create_surface_from_hwnd(hinstance as *mut _, hwnd as *mut _)
        }
    }
