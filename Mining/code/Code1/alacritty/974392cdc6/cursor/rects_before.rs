    fn rects(self, size_info: &SizeInfo, thickness: f32) -> CursorRects {
        let point = self.point();
        let x = point.column.0 as f32 * size_info.cell_width() + size_info.padding_x();
        let y = point.line.0 as f32 * size_info.cell_height() + size_info.padding_y();

        let mut width = size_info.cell_width();
        let height = size_info.cell_height();

        if self.is_wide() {
            width *= 2.;
        }

        let thickness = (thickness * width as f32).round().max(1.);

        match self.shape() {
            CursorShape::Beam => beam(x, y, height, thickness, self.color()),
            CursorShape::Underline => underline(x, y, width, height, thickness, self.color()),
            CursorShape::HollowBlock => hollow(x, y, width, height, thickness, self.color()),
            _ => CursorRects::default(),
        }
    }
