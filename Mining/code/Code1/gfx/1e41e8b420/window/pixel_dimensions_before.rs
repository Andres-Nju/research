    fn pixel_dimensions(&self) -> (image::Size, image::Size) {
        unsafe {
            // NSView bounds are measured in DIPs
            let bounds: NSRect = msg_send![self.0.nsview, bounds];
            (bounds.size.width as _, bounds.size.height as _)
        }
    }
