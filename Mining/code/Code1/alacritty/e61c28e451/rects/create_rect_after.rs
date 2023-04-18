    fn create_rect(
        metrics: &Metrics,
        size: &SizeInfo,
        flag: Flags,
        start: Point,
        end: Point,
        color: Rgb,
    ) -> RenderRect {
        let start_x = start.col.0 as f32 * size.cell_width;
        let end_x = (end.col.0 + 1) as f32 * size.cell_width;
        let width = end_x - start_x;

        let (position, mut height) = match flag {
            Flags::UNDERLINE => (metrics.underline_position, metrics.underline_thickness),
            Flags::STRIKEOUT => (metrics.strikeout_position, metrics.strikeout_thickness),
            _ => unimplemented!("Invalid flag for cell line drawing specified"),
        };

        // Make sure lines are always visible
        height = height.max(1.);

        let line_bottom = (start.line.0 as f32 + 1.) * size.cell_height;
        let baseline = line_bottom + metrics.descent;

        let mut y = (baseline - position - height / 2.).ceil();
        let max_y = line_bottom - height;
        if y > max_y {
            y = max_y;
        }

        RenderRect::new(start_x + size.padding_x, y + size.padding_y, width, height, color, 1.)
    }
