    fn from(c: [f32; 4]) -> PackedColor {
        PackedColor(c.iter().rev().fold(0, |u, &c| {
            (u<<8) + (c * 255.0 + 0.5) as u32
        }))
    }
