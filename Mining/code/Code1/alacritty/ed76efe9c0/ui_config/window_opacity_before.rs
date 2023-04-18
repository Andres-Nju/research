    pub fn window_opacity(&self) -> f32 {
        self.window_opacity.unwrap_or(self.window.opacity).as_f32()
    }
