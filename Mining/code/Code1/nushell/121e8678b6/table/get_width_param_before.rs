fn get_width_param(width_param: Option<i64>) -> usize {
    if let Some(col) = width_param {
        col as usize
    } else if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
        (w - 1) as usize
    } else {
        80usize
    }
}
