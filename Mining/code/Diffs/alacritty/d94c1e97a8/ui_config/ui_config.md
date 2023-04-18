File_Code/alacritty/d94c1e97a8/ui_config/ui_config_after.rs --- Rust
26 const URL_REGEX: &str = "(magnet:|mailto:|gemini:|gopher:|https:|http:|news:|file:|git:|ssh:|ftp:)\                                                       26 const URL_REGEX: &str = "(magnet:|mailto:|gemini:|gopher:|https:|http:|news:|file:|git:|ssh:|ftp:)\
27                          [^\u{0000}-\u{001F}\u{007F}-\u{009F}<>\" {-}\\^⟨⟩`]+";                                                                           27                          [^\u{0000}-\u{001F}\u{007F}-\u{009F}<>\"\\s{-}\\^⟨⟩`]+";

