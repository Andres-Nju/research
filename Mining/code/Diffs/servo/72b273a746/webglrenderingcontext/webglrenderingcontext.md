File_Code/servo/72b273a746/webglrenderingcontext/webglrenderingcontext_after.rs --- Rust
1121         capture_stack!(in(obj.global().get_cx()) let stack);                                                                                            1121         capture_stack!(in(*obj.global().get_cx()) let stack);
1122         WebGLCommandBacktrace {                                                                                                                         1122         WebGLCommandBacktrace {
1123             backtrace: format!("{:?}", bt),                                                                                                             1123             backtrace: format!("{:?}", bt),
1124             js_backtrace: stack.and_then(|s| s.as_string(None)),                                                                                        1124             js_backtrace: stack.and_then(|s| s.as_string(None, js::jsapi::StackFormat::Default)),

