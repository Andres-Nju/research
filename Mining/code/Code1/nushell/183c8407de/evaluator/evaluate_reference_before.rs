fn evaluate_reference(
    name: &hir::Variable,
    scope: &Scope,
    source: &Text,
    tag: Tag,
) -> Result<Value, ShellError> {
    trace!("Evaluating {:?} with Scope {:?}", name, scope);
    match name {
        hir::Variable::It(_) => Ok(scope.it.value.clone().into_value(tag)),
        hir::Variable::Other(_, span) => match span.slice(source) {
            x if x == "nu" => crate::evaluate::variables::nu(tag),
            x => Ok(scope
                .vars
                .get(x)
                .cloned()
                .unwrap_or_else(|| UntaggedValue::nothing().into_value(tag))),
        },
    }
}
