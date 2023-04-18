fn parse_invocation(
    lite_arg: &Spanned<String>,
    scope: &dyn ParserScope,
) -> (SpannedExpression, Option<ParseError>) {
    // We have a command invocation
    let string: String = lite_arg
        .item
        .chars()
        .skip(2)
        .take(lite_arg.item.chars().count() - 3)
        .collect();

    // We haven't done much with the inner string, so let's go ahead and work with it
    let (tokens, err) = lex(&string, lite_arg.span.start() + 2);
    if err.is_some() {
        return (garbage(lite_arg.span), err);
    };
    let (lite_block, err) = parse_block(tokens);
    if err.is_some() {
        return (garbage(lite_arg.span), err);
    };

    scope.enter_scope();
    let (classified_block, err) = classify_block(&lite_block, scope);
    scope.exit_scope();

    (
        SpannedExpression::new(Expression::Invocation(classified_block), lite_arg.span),
        err,
    )
}
