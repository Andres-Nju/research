unsafe fn ime_set_context(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    // Don't show system CompositionWindow because application itself draws it
    let lparam = lparam & !ISC_SHOWUICOMPOSITIONWINDOW;
    let result = DefWindowProcW(hwnd, msg, wparam, lparam);
    Some(result)
}
