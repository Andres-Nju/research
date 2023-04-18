pub fn capture_webgl_backtrace<T: DomObject>(obj: &T) -> WebGLCommandBacktrace {
    let bt = Backtrace::new();
    unsafe {
        capture_stack!(in(*obj.global().get_cx()) let stack);
        WebGLCommandBacktrace {
            backtrace: format!("{:?}", bt),
            js_backtrace: stack.and_then(|s| s.as_string(None, js::jsapi::StackFormat::Default)),
        }
    }
}
