fn exists(path: &Path, span: Span, args: &Arguments) -> Value {
    let path = expand_path_with(path, &args.pwd);
    Value::Bool {
        val: path.exists(),
        span,
    }
}
