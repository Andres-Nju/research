File_Code/lapce/8b5eebaa53/tab_header_content/tab_header_content_after.rs --- Rust
575                 .take(length - skip_left - skip_right)                                                                                                   575                 .take(length.saturating_sub(skip_left).saturating_sub(skip_right))

