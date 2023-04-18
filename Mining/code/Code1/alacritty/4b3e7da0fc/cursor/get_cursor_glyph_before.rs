pub fn get_cursor_glyph(
    cursor: CursorStyle,
    metrics: Metrics,
    offset_x: i8,
    offset_y: i8,
    is_wide: bool,
) -> RasterizedGlyph {
    // Calculate the cell metrics
    let height = metrics.line_height as i32 + i32::from(offset_y);
    let mut width = metrics.average_advance as i32 + i32::from(offset_x);
    let line_width = cmp::max(width * CURSOR_WIDTH_PERCENTAGE / 100, 1);

    // Double the cursor width if it's above a double-width glyph
    if is_wide {
        width *= 2;
    }

    match cursor {
        CursorStyle::HollowBlock => get_box_cursor_glyph(height, width, line_width),
        CursorStyle::Underline => get_underline_cursor_glyph(width, line_width),
        CursorStyle::Beam => get_beam_cursor_glyph(height, line_width),
        CursorStyle::Block => get_block_cursor_glyph(height, width),
    }
}
