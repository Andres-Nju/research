pub fn parse(math_expression: &str, tag: impl Into<Tag>) -> Result<Value, String> {
    let num = meval::eval_str(math_expression);
    match num {
        Ok(num) => Ok(UntaggedValue::from(Primitive::from(num)).into_value(tag)),
        Err(error) => Err(error.to_string()),
    }
}
