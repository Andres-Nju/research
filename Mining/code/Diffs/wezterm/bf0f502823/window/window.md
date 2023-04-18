File_Code/wezterm/bf0f502823/window/window_after.rs --- 1/2 --- Rust
52 const ISC_SHOWUICOMPOSITIONWINDOW: LPARAM = 2147483648;                                                                                                   52 const ISC_SHOWUICOMPOSITIONWINDOW: DWORD = 0x80000000;

File_Code/wezterm/bf0f502823/window/window_after.rs --- 2/2 --- Rust
1731     let lparam = lparam & !ISC_SHOWUICOMPOSITIONWINDOW;                                                                                                 1731     let lparam = lparam & !(ISC_SHOWUICOMPOSITIONWINDOW as LPARAM);

