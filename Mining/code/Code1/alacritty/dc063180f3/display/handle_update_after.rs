    pub fn handle_update<T>(
        &mut self,
        terminal: &mut Term<T>,
        pty_resize_handle: &mut dyn OnResize,
        message_buffer: &MessageBuffer,
        config: &Config,
        update_pending: DisplayUpdate,
    ) {
        // Update font size and cell dimensions
        if let Some(font) = update_pending.font {
            self.update_glyph_cache(config, font);
        }

        let cell_width = self.size_info.cell_width;
        let cell_height = self.size_info.cell_height;

        // Recalculate padding
        let mut padding_x = f32::from(config.window.padding.x) * self.size_info.dpr as f32;
        let mut padding_y = f32::from(config.window.padding.y) * self.size_info.dpr as f32;

        // Update the window dimensions
        if let Some(size) = update_pending.dimensions {
            // Ensure we have at least one column and row
            self.size_info.width = (size.width as f32).max(cell_width + 2. * padding_x);
            self.size_info.height = (size.height as f32).max(cell_height + 2. * padding_y);
        }

        // Distribute excess padding equally on all sides
        if config.window.dynamic_padding {
            padding_x = dynamic_padding(padding_x, self.size_info.width, cell_width);
            padding_y = dynamic_padding(padding_y, self.size_info.height, cell_height);
        }

        self.size_info.padding_x = padding_x.floor() as f32;
        self.size_info.padding_y = padding_y.floor() as f32;

        let mut pty_size = self.size_info;

        // Subtract message bar lines from pty size
        if let Some(message) = message_buffer.message() {
            let lines = message.text(&self.size_info).len();
            pty_size.height -= pty_size.cell_height * lines as f32;
        }

        // Resize PTY
        pty_resize_handle.on_resize(&pty_size);

        // Resize terminal
        terminal.resize(&pty_size);

        // Resize renderer
        let physical =
            PhysicalSize::new(f64::from(self.size_info.width), f64::from(self.size_info.height));
        self.window.resize(physical);
        self.renderer.resize(&self.size_info);
    }
