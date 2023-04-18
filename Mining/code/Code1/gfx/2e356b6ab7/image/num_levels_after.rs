    pub fn num_levels(&self) -> Level {
        use std::cmp::max;
        match *self {
            Kind::D2(_, _, _, s) if s > 1 => {
                // anti-aliased images can't have mipmaps
                1
            }
            _ => {
                let extent = self.extent();
                let dominant = max(max(extent.width, extent.height), extent.depth);
                (1..).find(|level| dominant>>level == 0).unwrap()
            }
        }
    }
