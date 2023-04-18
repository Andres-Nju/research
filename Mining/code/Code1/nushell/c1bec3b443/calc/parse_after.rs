pub fn parse(math_expression: &str, tag: impl Into<Tag>) -> Result<Value, String> {
    use std::f64;
    let num = meval::eval_str(math_expression);
    match num {
        Ok(num) => {
            if num == f64::INFINITY || num == f64::NEG_INFINITY {
                return Err(String::from("cannot represent result"));
            }
            Ok(UntaggedValue::from(Primitive::from(num)).into_value(tag))
        }
        Err(error) => Err(error.to_string()),
    }
}
