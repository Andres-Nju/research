    fn new(config: &Config, colors: &color::List, cell: Indexed<Cell>, selected: bool) -> Self {
        // Lookup RGB values
        let mut fg_rgb = Self::compute_fg_rgb(config, colors, cell.fg, cell.flags);
        let mut bg_rgb = Self::compute_bg_rgb(colors, cell.bg);
        let mut bg_alpha = Self::compute_bg_alpha(cell.bg);

        let selection_background = config.colors.selection.background;
        if let (true, Some(col)) = (selected, selection_background) {
            // Override selection background with config colors
            bg_rgb = col;
            bg_alpha = 1.0;
        } else if selected ^ cell.inverse() {
            if fg_rgb == bg_rgb && !cell.flags.contains(Flags::HIDDEN) {
                // Reveal inversed text when fg/bg is the same
                fg_rgb = colors[NamedColor::Background];
                bg_rgb = colors[NamedColor::Foreground];
            } else {
                // Invert cell fg and bg colors
                mem::swap(&mut fg_rgb, &mut bg_rgb);
            }
        }

        // Override selection text with config colors
        if let (true, Some(col)) = (selected, config.colors.selection.text) {
            fg_rgb = col;
        }

        RenderableCell {
            line: cell.line,
            column: cell.column,
            inner: RenderableCellContent::Chars(cell.chars()),
            fg: fg_rgb,
            bg: bg_rgb,
            bg_alpha,
            flags: cell.flags,
        }
    }
