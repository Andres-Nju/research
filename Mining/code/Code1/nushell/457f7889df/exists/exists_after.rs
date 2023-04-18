fn exists(path: &Path, span: Span, args: &Arguments) -> Value {
    let path = expand_path_with(path, &args.pwd);
    Value::Bool {
        val: match path.try_exists() {
            Ok(exists) => exists,
            Err(err) => return Value::Error { error: err.into() },
        },
        span,
    }
}
