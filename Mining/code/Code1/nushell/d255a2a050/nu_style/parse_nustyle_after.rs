pub fn parse_nustyle(nu_style: NuStyle) -> Style {
    // get the nu_ansi_term::Color foreground color
    let fg_color = match nu_style.fg {
        Some(fg) => color_from_hex(&fg).unwrap_or_default(),
        _ => None,
    };
    // get the nu_ansi_term::Color background color
    let bg_color = match nu_style.bg {
        Some(bg) => color_from_hex(&bg).unwrap_or_default(),
        _ => None,
    };
    // get the attributes
    let color_attr = match nu_style.attr {
        Some(attr) => attr,
        _ => "".to_string(),
    };

    // setup the attributes available in nu_ansi_term::Style
    let mut bold = false;
    let mut dimmed = false;
    let mut italic = false;
    let mut underline = false;
    let mut blink = false;
    let mut reverse = false;
    let mut hidden = false;
    let mut strikethrough = false;

    // since we can combine styles like bold-italic, iterate through the chars
    // and set the bools for later use in the nu_ansi_term::Style application
    for ch in color_attr.to_lowercase().chars() {
        match ch {
            'l' => blink = true,
            'b' => bold = true,
            'd' => dimmed = true,
            'h' => hidden = true,
            'i' => italic = true,
            'r' => reverse = true,
            's' => strikethrough = true,
            'u' => underline = true,
            'n' => (),
            _ => (),
        }
    }

    // here's where we build the nu_ansi_term::Style
    Style {
        foreground: fg_color,
        background: bg_color,
        is_blink: blink,
        is_bold: bold,
        is_dimmed: dimmed,
        is_hidden: hidden,
        is_italic: italic,
        is_reverse: reverse,
        is_strikethrough: strikethrough,
        is_underline: underline,
    }
}
