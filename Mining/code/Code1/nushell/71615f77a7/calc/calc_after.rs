fn calc(input: Value, args: &RawCommandArgs) -> Result<OutputStream, ShellError> {
    let name_span = &args.call_info.name_tag.span;

    let output = if let Ok(string) = input.as_string() {
        match parse(&string, &input.tag) {
            Ok(value) => ReturnSuccess::value(value),
            Err(err) => Err(ShellError::labeled_error(
                "Calculation error",
                err,
                &input.tag.span,
            )),
        }
    } else {
        Err(ShellError::labeled_error(
            "Expected a string from pipeline",
            "requires string input",
            name_span,
        ))
    };

    Ok(vec![output].into())
}
