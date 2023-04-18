fn check_insert(
    existing: &mut ShapeMap,
    to_add: (String, (Span, Option<SyntaxShape>)),
) -> Result<(), ShellError> {
    match (to_add.1).1 {
        None => match existing.get(&to_add.0) {
            None => {
                existing.insert(to_add.0, to_add.1);
                Ok(())
            }
            Some(_) => Ok(()),
        },
        Some(new) => match existing.insert(to_add.0.clone(), ((to_add.1).0, Some(new))) {
            None => Ok(()),
            Some(exist) => match exist.1 {
                None => Ok(()),
                Some(shape) => match shape {
                    SyntaxShape::Any => Ok(()),
                    shape if shape == new => Ok(()),
                    _ => Err(ShellError::labeled_error_with_secondary(
                        "Type conflict in alias variable use",
                        format!("{:?}", new),
                        (to_add.1).0,
                        format!("{:?}", shape),
                        exist.0,
                    )),
                },
            },
        },
    }
}
