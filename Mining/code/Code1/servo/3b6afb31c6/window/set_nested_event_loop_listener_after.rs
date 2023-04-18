    pub unsafe fn set_nested_event_loop_listener(
            &self,
            listener: *mut (NestedEventLoopListener + 'static)) {
        g_nested_event_loop_listener = Some(listener)
    }

    pub unsafe fn remove_nested_event_loop_listener(&self) {
        g_nested_event_loop_listener = None
    }

    fn glutin_key_to_script_key(key: glutin::VirtualKeyCode) -> Result<constellation_msg::Key, ()> {
        // TODO(negge): add more key mappings
        match key {
            VirtualKeyCode::A => Ok(Key::A),
            VirtualKeyCode::B => Ok(Key::B),
            VirtualKeyCode::C => Ok(Key::C),
            VirtualKeyCode::D => Ok(Key::D),
            VirtualKeyCode::E => Ok(Key::E),
            VirtualKeyCode::F => Ok(Key::F),
            VirtualKeyCode::G => Ok(Key::G),
            VirtualKeyCode::H => Ok(Key::H),
            VirtualKeyCode::I => Ok(Key::I),
            VirtualKeyCode::J => Ok(Key::J),
            VirtualKeyCode::K => Ok(Key::K),
            VirtualKeyCode::L => Ok(Key::L),
            VirtualKeyCode::M => Ok(Key::M),
            VirtualKeyCode::N => Ok(Key::N),
            VirtualKeyCode::O => Ok(Key::O),
            VirtualKeyCode::P => Ok(Key::P),
            VirtualKeyCode::Q => Ok(Key::Q),
            VirtualKeyCode::R => Ok(Key::R),
            VirtualKeyCode::S => Ok(Key::S),
            VirtualKeyCode::T => Ok(Key::T),
            VirtualKeyCode::U => Ok(Key::U),
            VirtualKeyCode::V => Ok(Key::V),
            VirtualKeyCode::W => Ok(Key::W),
            VirtualKeyCode::X => Ok(Key::X),
            VirtualKeyCode::Y => Ok(Key::Y),
            VirtualKeyCode::Z => Ok(Key::Z),

            VirtualKeyCode::Numpad0 => Ok(Key::Kp0),
            VirtualKeyCode::Numpad1 => Ok(Key::Kp1),
            VirtualKeyCode::Numpad2 => Ok(Key::Kp2),
            VirtualKeyCode::Numpad3 => Ok(Key::Kp3),
            VirtualKeyCode::Numpad4 => Ok(Key::Kp4),
            VirtualKeyCode::Numpad5 => Ok(Key::Kp5),
            VirtualKeyCode::Numpad6 => Ok(Key::Kp6),
            VirtualKeyCode::Numpad7 => Ok(Key::Kp7),
            VirtualKeyCode::Numpad8 => Ok(Key::Kp8),
            VirtualKeyCode::Numpad9 => Ok(Key::Kp9),

            VirtualKeyCode::Key0 => Ok(Key::Num0),
            VirtualKeyCode::Key1 => Ok(Key::Num1),
            VirtualKeyCode::Key2 => Ok(Key::Num2),
            VirtualKeyCode::Key3 => Ok(Key::Num3),
            VirtualKeyCode::Key4 => Ok(Key::Num4),
            VirtualKeyCode::Key5 => Ok(Key::Num5),
            VirtualKeyCode::Key6 => Ok(Key::Num6),
            VirtualKeyCode::Key7 => Ok(Key::Num7),
            VirtualKeyCode::Key8 => Ok(Key::Num8),
            VirtualKeyCode::Key9 => Ok(Key::Num9),

            VirtualKeyCode::Return => Ok(Key::Enter),
            VirtualKeyCode::Space => Ok(Key::Space),
            VirtualKeyCode::Escape => Ok(Key::Escape),
            VirtualKeyCode::Equals => Ok(Key::Equal),
            VirtualKeyCode::Minus => Ok(Key::Minus),
            VirtualKeyCode::Back => Ok(Key::Backspace),
            VirtualKeyCode::PageDown => Ok(Key::PageDown),
            VirtualKeyCode::PageUp => Ok(Key::PageUp),

            VirtualKeyCode::Insert => Ok(Key::Insert),
            VirtualKeyCode::Home => Ok(Key::Home),
            VirtualKeyCode::Delete => Ok(Key::Delete),
            VirtualKeyCode::End => Ok(Key::End),

            VirtualKeyCode::Left => Ok(Key::Left),
            VirtualKeyCode::Up => Ok(Key::Up),
            VirtualKeyCode::Right => Ok(Key::Right),
            VirtualKeyCode::Down => Ok(Key::Down),

            VirtualKeyCode::Apostrophe => Ok(Key::Apostrophe),
            VirtualKeyCode::Backslash => Ok(Key::Backslash),
            VirtualKeyCode::Comma => Ok(Key::Comma),
            VirtualKeyCode::Grave => Ok(Key::GraveAccent),
            VirtualKeyCode::LBracket => Ok(Key::LeftBracket),
            VirtualKeyCode::Period => Ok(Key::Period),
            VirtualKeyCode::RBracket => Ok(Key::RightBracket),
            VirtualKeyCode::Semicolon => Ok(Key::Semicolon),
            VirtualKeyCode::Slash => Ok(Key::Slash),
            VirtualKeyCode::Tab => Ok(Key::Tab),
            VirtualKeyCode::Subtract => Ok(Key::Minus),

            VirtualKeyCode::NavigateBackward => Ok(Key::NavigateBackward),
            VirtualKeyCode::NavigateForward => Ok(Key::NavigateForward),
            _ => Err(()),
        }
    }

    fn glutin_mods_to_script_mods(modifiers: KeyModifiers) -> constellation_msg::KeyModifiers {
        let mut result = constellation_msg::KeyModifiers::from_bits(0).unwrap();
        if modifiers.intersects(LEFT_SHIFT | RIGHT_SHIFT) {
            result.insert(SHIFT);
        }
        if modifiers.intersects(LEFT_CONTROL | RIGHT_CONTROL) {
            result.insert(CONTROL);
        }
        if modifiers.intersects(LEFT_ALT | RIGHT_ALT) {
            result.insert(ALT);
        }
        if modifiers.intersects(LEFT_SUPER | RIGHT_SUPER) {
            result.insert(SUPER);
        }
        result
    }

    #[cfg(all(feature = "window", not(target_os = "win")))]
    fn platform_handle_key(&self, key: Key, mods: constellation_msg::KeyModifiers) {
        match (mods, key) {
            (CMD_OR_CONTROL, Key::LeftBracket) => {
                self.event_queue.borrow_mut().push(WindowEvent::Navigation(WindowNavigateMsg::Back));
            }
            (CMD_OR_CONTROL, Key::RightBracket) => {
                self.event_queue.borrow_mut().push(WindowEvent::Navigation(WindowNavigateMsg::Forward));
            }
            _ => {}
        }
    }

    #[cfg(all(feature = "window", target_os = "win"))]
    fn platform_handle_key(&self, key: Key, mods: constellation_msg::KeyModifiers) {
    }
}

// WindowProxy is not implemented for android yet

#[cfg(all(feature = "window", target_os = "android"))]
fn create_window_proxy(_: &Rc<Window>) -> Option<glutin::WindowProxy> {
    None
}

#[cfg(all(feature = "window", not(target_os = "android")))]
fn create_window_proxy(window: &Rc<Window>) -> Option<glutin::WindowProxy> {
    Some(window.window.create_window_proxy())
}

#[cfg(feature = "window")]
impl WindowMethods for Window {
    fn framebuffer_size(&self) -> TypedSize2D<DevicePixel, u32> {
        let scale_factor = self.window.hidpi_factor() as u32;
        let (width, height) = self.window.get_inner_size().unwrap();
        Size2D::typed(width * scale_factor, height * scale_factor)
    }

    fn size(&self) -> TypedSize2D<ScreenPx, f32> {
        let (width, height) = self.window.get_inner_size().unwrap();
        Size2D::typed(width as f32, height as f32)
    }

    fn client_window(&self) -> (Size2D<u32>, Point2D<i32>) {
        let (width, height) = self.window.get_outer_size().unwrap();
        let size = Size2D::new(width, height);
        let (x, y) = self.window.get_position().unwrap();
        let origin = Point2D::new(x as i32, y as i32);
        (size, origin)
    }

    fn set_inner_size(&self, size: Size2D<u32>) {
        self.window.set_inner_size(size.width as u32, size.height as u32)
    }

    fn set_position(&self, point: Point2D<i32>) {
        self.window.set_position(point.x, point.y)
    }

    fn present(&self) {
        self.window.swap_buffers().unwrap();
    }

    fn create_compositor_channel(window: &Option<Rc<Window>>)
                                 -> (Box<CompositorProxy + Send>, Box<CompositorReceiver>) {
        let (sender, receiver) = channel();

        let window_proxy = match window {
            &Some(ref window) => create_window_proxy(window),
            &None => None,
        };

        (box GlutinCompositorProxy {
             sender: sender,
             window_proxy: window_proxy,
         } as Box<CompositorProxy + Send>,
         box receiver as Box<CompositorReceiver>)
    }

    fn hidpi_factor(&self) -> ScaleFactor<ScreenPx, DevicePixel, f32> {
        ScaleFactor::new(self.window.hidpi_factor())
    }

    fn set_page_title(&self, title: Option<String>) {
        let title = match title {
            Some(ref title) if title.len() > 0 => &**title,
            _ => "untitled",
        };
        let title = format!("{} - Servo", title);
        self.window.set_title(&title);
    }

    fn set_page_url(&self, _: Url) {
    }

    fn status(&self, _: Option<String>) {
    }

    fn load_start(&self, _: bool, _: bool) {
    }

    fn load_end(&self, _: bool, _: bool, root: bool) {
        if root && opts::get().no_native_titlebar {
            self.window.show()
        }
    }

    fn load_error(&self, _: NetError, _: String) {
    }

    fn head_parsed(&self) {
    }

    /// Has no effect on Android.
    fn set_cursor(&self, c: Cursor) {
        use glutin::MouseCursor;

        let glutin_cursor = match c {
            Cursor::NoCursor => MouseCursor::NoneCursor,
            Cursor::DefaultCursor => MouseCursor::Default,
            Cursor::PointerCursor => MouseCursor::Hand,
            Cursor::ContextMenuCursor => MouseCursor::ContextMenu,
            Cursor::HelpCursor => MouseCursor::Help,
            Cursor::ProgressCursor => MouseCursor::Progress,
            Cursor::WaitCursor => MouseCursor::Wait,
            Cursor::CellCursor => MouseCursor::Cell,
            Cursor::CrosshairCursor => MouseCursor::Crosshair,
            Cursor::TextCursor => MouseCursor::Text,
            Cursor::VerticalTextCursor => MouseCursor::VerticalText,
            Cursor::AliasCursor => MouseCursor::Alias,
            Cursor::CopyCursor => MouseCursor::Copy,
            Cursor::MoveCursor => MouseCursor::Move,
            Cursor::NoDropCursor => MouseCursor::NoDrop,
            Cursor::NotAllowedCursor => MouseCursor::NotAllowed,
            Cursor::GrabCursor => MouseCursor::Grab,
            Cursor::GrabbingCursor => MouseCursor::Grabbing,
            Cursor::EResizeCursor => MouseCursor::EResize,
            Cursor::NResizeCursor => MouseCursor::NResize,
            Cursor::NeResizeCursor => MouseCursor::NeResize,
            Cursor::NwResizeCursor => MouseCursor::NwResize,
            Cursor::SResizeCursor => MouseCursor::SResize,
            Cursor::SeResizeCursor => MouseCursor::SeResize,
            Cursor::SwResizeCursor => MouseCursor::SwResize,
            Cursor::WResizeCursor => MouseCursor::WResize,
            Cursor::EwResizeCursor => MouseCursor::EwResize,
            Cursor::NsResizeCursor => MouseCursor::NsResize,
            Cursor::NeswResizeCursor => MouseCursor::NeswResize,
            Cursor::NwseResizeCursor => MouseCursor::NwseResize,
            Cursor::ColResizeCursor => MouseCursor::ColResize,
            Cursor::RowResizeCursor => MouseCursor::RowResize,
            Cursor::AllScrollCursor => MouseCursor::AllScroll,
            Cursor::ZoomInCursor => MouseCursor::ZoomIn,
            Cursor::ZoomOutCursor => MouseCursor::ZoomOut,
        };
        self.window.set_cursor(glutin_cursor);
    }

    fn set_favicon(&self, _: Url) {
    }

    fn prepare_for_composite(&self, _width: usize, _height: usize) -> bool {
        true
    }

    #[cfg(target_os = "linux")]
    fn native_display(&self) -> NativeDisplay {
        use x11::xlib;
        unsafe {
            match opts::get().render_api {
                RenderApi::GL => {
                    NativeDisplay::new(self.window.platform_display() as *mut xlib::Display)
                },
                RenderApi::ES2 => {
                    NativeDisplay::new_egl_display()
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn native_display(&self) -> NativeDisplay {
        NativeDisplay::new()
    }

    /// Helper function to handle keyboard events.
    fn handle_key(&self, key: Key, mods: constellation_msg::KeyModifiers) {

        match (mods, key) {
            (_, Key::Equal) => {
                if mods & !SHIFT == CMD_OR_CONTROL {
                    self.event_queue.borrow_mut().push(WindowEvent::Zoom(1.1));
                } else if mods & !SHIFT == CMD_OR_CONTROL | ALT {
                    self.event_queue.borrow_mut().push(WindowEvent::PinchZoom(1.1));
                }
            }
            (CMD_OR_CONTROL, Key::Minus) => {
                self.event_queue.borrow_mut().push(WindowEvent::Zoom(1.0 / 1.1));
            }
            (_, Key::Minus) if mods == CMD_OR_CONTROL | ALT => {
                self.event_queue.borrow_mut().push(WindowEvent::PinchZoom(1.0 / 1.1));
            }
            (CMD_OR_CONTROL, Key::Num0) |
            (CMD_OR_CONTROL, Key::Kp0) => {
                self.event_queue.borrow_mut().push(WindowEvent::ResetZoom);
            }

            (NONE, Key::NavigateForward) => {
                self.event_queue.borrow_mut().push(WindowEvent::Navigation(WindowNavigateMsg::Forward));
            }
            (NONE, Key::NavigateBackward) => {
                self.event_queue.borrow_mut().push(WindowEvent::Navigation(WindowNavigateMsg::Back));
            }

            (NONE, Key::Escape) => {
                self.event_queue.borrow_mut().push(WindowEvent::Quit);
            }

            (CMD_OR_ALT, Key::Right) => {
                self.event_queue.borrow_mut().push(WindowEvent::Navigation(WindowNavigateMsg::Forward));
            }
            (CMD_OR_ALT, Key::Left) => {
                self.event_queue.borrow_mut().push(WindowEvent::Navigation(WindowNavigateMsg::Back));
            }

            (NONE, Key::PageDown) |
            (NONE, Key::Space) => {
                self.scroll_window(0.0,
                                   -self.framebuffer_size()
                                        .as_f32()
                                        .to_untyped()
                                        .height + 2.0 * LINE_HEIGHT,
                                   TouchEventType::Move);
            }
            (NONE, Key::PageUp) |
            (SHIFT, Key::Space) => {
                self.scroll_window(0.0,
                                   self.framebuffer_size()
                                       .as_f32()
                                       .to_untyped()
                                       .height - 2.0 * LINE_HEIGHT,
                                   TouchEventType::Move);
            }
            (NONE, Key::Up) => {
                self.scroll_window(0.0, 3.0 * LINE_HEIGHT, TouchEventType::Move);
            }
            (NONE, Key::Down) => {
                self.scroll_window(0.0, -3.0 * LINE_HEIGHT, TouchEventType::Move);
            }
            (NONE, Key::Left) => {
                self.scroll_window(LINE_HEIGHT, 0.0, TouchEventType::Move);
            }
            (NONE, Key::Right) => {
                self.scroll_window(-LINE_HEIGHT, 0.0, TouchEventType::Move);
            }

            _ => {
                self.platform_handle_key(key, mods);
            }
        }
    }

    fn supports_clipboard(&self) -> bool {
        true
    }
}

/// The type of a window.
#[cfg(feature = "headless")]
pub struct Window {
    #[allow(dead_code)]
    context: glutin::HeadlessContext,
    width: u32,
    height: u32,
}

#[cfg(feature = "headless")]
impl Window {
    pub fn new(_is_foreground: bool,
               window_size: TypedSize2D<DevicePixel, u32>,
               _parent: Option<glutin::WindowID>) -> Rc<Window> {
        let window_size = window_size.to_untyped();
        let headless_builder = glutin::HeadlessRendererBuilder::new(window_size.width,
                                                                    window_size.height);
        let headless_context = headless_builder.build().unwrap();
        unsafe { headless_context.make_current().expect("Failed to make context current!") };

        gl::load_with(|s| headless_context.get_proc_address(s) as *const c_void);

        let window = Window {
            context: headless_context,
            width: window_size.width,
            height: window_size.height,
        };

        Rc::new(window)
    }

    pub fn wait_events(&self) -> Vec<WindowEvent> {
        vec![WindowEvent::Idle]
    }

    pub unsafe fn set_nested_event_loop_listener(
            &self,
            _listener: *mut (NestedEventLoopListener + 'static)) {
    }

    pub unsafe fn remove_nested_event_loop_listener(&self) {
    }
}

#[cfg(feature = "headless")]
