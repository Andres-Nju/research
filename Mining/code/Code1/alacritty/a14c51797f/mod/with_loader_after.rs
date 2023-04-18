    pub fn with_loader<F, T>(&mut self, func: F) -> T
    where
        F: FnOnce(LoaderApi<'_>) -> T,
    {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
        }

        func(LoaderApi {
            active_tex: &mut self.active_tex,
            atlas: &mut self.atlas,
            current_atlas: &mut self.current_atlas,
        })
    }

    pub fn reload_shaders(&mut self, props: &term::SizeInfo) {
        info!("Reloading shaders...");
        let result = (TextShaderProgram::new(), RectShaderProgram::new());
        let (program, rect_program) = match result {
            (Ok(program), Ok(rect_program)) => {
                unsafe {
                    gl::UseProgram(program.id);
                    program.update_projection(
                        props.width,
                        props.height,
                        props.padding_x,
                        props.padding_y,
                    );
                    gl::UseProgram(0);
                }

                info!("... successfully reloaded shaders");
                (program, rect_program)
            },
            (Err(err), _) | (_, Err(err)) => {
                error!("{}", err);
                return;
            },
        };

        self.active_tex = 0;
        self.program = program;
        self.rect_program = rect_program;
    }

    pub fn resize(&mut self, size: &SizeInfo) {
        // Viewport.
        unsafe {
            gl::Viewport(
                size.padding_x as i32,
                size.padding_y as i32,
                size.width as i32 - 2 * size.padding_x as i32,
                size.height as i32 - 2 * size.padding_y as i32,
            );

            // Update projection.
            gl::UseProgram(self.program.id);
            self.program.update_projection(size.width, size.height, size.padding_x, size.padding_y);
            gl::UseProgram(0);
        }
    }

    /// Render a rectangle.
    ///
    /// This requires the rectangle program to be activated.
    fn render_rect(&mut self, rect: &RenderRect, size: &term::SizeInfo) {
        // Do nothing when alpha is fully transparent.
        if rect.alpha == 0. {
            return;
        }

        // Calculate rectangle position.
        let center_x = size.width / 2.;
        let center_y = size.height / 2.;
        let x = (rect.x - center_x) / center_x;
        let y = -(rect.y - center_y) / center_y;
        let width = rect.width / center_x;
        let height = rect.height / center_y;

        unsafe {
            // Setup vertices.
            let vertices: [f32; 8] = [x + width, y, x + width, y - height, x, y - height, x, y];

            // Load vertex data into array buffer.
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (size_of::<f32>() * vertices.len()) as _,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Color.
            self.rect_program.set_color(rect.color, rect.alpha);

            // Draw the rectangle.
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, ptr::null());
        }
    }
}

impl<'a, C> RenderApi<'a, C> {
    pub fn clear(&self, color: Rgb) {
        unsafe {
            let alpha = self.config.background_opacity();
            gl::ClearColor(
                (f32::from(color.r) / 255.0).min(1.0) * alpha,
                (f32::from(color.g) / 255.0).min(1.0) * alpha,
                (f32::from(color.b) / 255.0).min(1.0) * alpha,
                alpha,
            );
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    pub fn finish(&self) {
        unsafe {
            gl::Finish();
        }
    }

    fn render_batch(&mut self) {
        unsafe {
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                self.batch.size() as isize,
                self.batch.instances.as_ptr() as *const _,
            );
        }

        // Bind texture if necessary.
        if *self.active_tex != self.batch.tex {
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, self.batch.tex);
            }
            *self.active_tex = self.batch.tex;
        }

        unsafe {
            self.program.set_background_pass(true);
            gl::DrawElementsInstanced(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                ptr::null(),
                self.batch.len() as GLsizei,
            );
            self.program.set_background_pass(false);
            gl::DrawElementsInstanced(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                ptr::null(),
                self.batch.len() as GLsizei,
            );
        }

        self.batch.clear();
    }

    /// Render a string in a variable location. Used for printing the render timer, warnings and
    /// errors.
    pub fn render_string(
        &mut self,
        string: &str,
        line: Line,
        glyph_cache: &mut GlyphCache,
        color: Option<Rgb>,
    ) {
        let bg_alpha = color.map(|_| 1.0).unwrap_or(0.0);
        let col = Column(0);

        let cells = string
            .chars()
            .enumerate()
            .map(|(i, c)| RenderableCell {
                line,
                column: col + i,
                inner: RenderableCellContent::Chars({
                    let mut chars = [' '; cell::MAX_ZEROWIDTH_CHARS + 1];
                    chars[0] = c;
                    chars
                }),
                bg: color.unwrap_or(Rgb { r: 0, g: 0, b: 0 }),
                fg: Rgb { r: 0, g: 0, b: 0 },
                flags: Flags::empty(),
                bg_alpha,
            })
            .collect::<Vec<_>>();

        for cell in cells {
            self.render_cell(cell, glyph_cache);
        }
    }

    #[inline]
    fn add_render_item(&mut self, cell: RenderableCell, glyph: &Glyph) {
        // Flush batch if tex changing.
        if !self.batch.is_empty() && self.batch.tex != glyph.tex_id {
            self.render_batch();
        }

        self.batch.add_item(cell, glyph);

        // Render batch and clear if it's full.
        if self.batch.full() {
            self.render_batch();
        }
