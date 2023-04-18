    fn into(self) -> [f32; 4] {
        let mut out = [0.0; 4];
        for i in 0 .. 4 {
            let byte = (self.0 >> (i<<3)) & 0xFF;
            out[i] = byte as f32 / 255.0;
        }
        out
    }
