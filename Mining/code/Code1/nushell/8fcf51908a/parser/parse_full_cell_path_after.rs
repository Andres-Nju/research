pub fn parse_full_cell_path(
    working_set: &mut StateWorkingSet,
    implicit_head: Option<VarId>,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let full_cell_span = span;
    let source = working_set.get_span_contents(span);
    let mut error = None;

    let (tokens, err) = lex(source, span.start, &[b'\n', b'\r'], &[b'.'], true);
    error = error.or(err);

    let mut tokens = tokens.into_iter().peekable();
    if let Some(head) = tokens.peek() {
        let bytes = working_set.get_span_contents(head.span);
        let (head, expect_dot) = if bytes.starts_with(b"(") {
            trace!("parsing: paren-head of full cell path");

            let head_span = head.span;
            let mut start = head.span.start;
            let mut end = head.span.end;

            if bytes.starts_with(b"(") {
                start += 1;
            }
            if bytes.ends_with(b")") {
                end -= 1;
            } else {
                error = error
                    .or_else(|| Some(ParseError::Unclosed(")".into(), Span { start: end, end })));
            }

            let span = Span { start, end };

            let source = working_set.get_span_contents(span);

            let (output, err) = lex(source, span.start, &[b'\n', b'\r'], &[], true);
            error = error.or(err);

            let (output, err) = lite_parse(&output);
            error = error.or(err);

            let (output, err) = parse_block(working_set, &output, true);
            error = error.or(err);

            let block_id = working_set.add_block(output);
            tokens.next();

            (
                Expression {
                    expr: Expr::Subexpression(block_id),
                    span: head_span,
                    ty: Type::Unknown, // FIXME
                    custom_completion: None,
                },
                true,
            )
        } else if bytes.starts_with(b"[") {
            trace!("parsing: table head of full cell path");

            let (output, err) = parse_table_expression(working_set, head.span);
            error = error.or(err);

            tokens.next();

            (output, true)
        } else if bytes.starts_with(b"{") {
            trace!("parsing: record head of full cell path");
            let (output, err) = parse_record(working_set, head.span);
            error = error.or(err);

            tokens.next();

            (output, true)
        } else if bytes.starts_with(b"$") {
            trace!("parsing: $variable head of full cell path");

            let (out, err) = parse_variable_expr(working_set, head.span);
            error = error.or(err);

            tokens.next();

            (out, true)
        } else if let Some(var_id) = implicit_head {
            (
                Expression {
                    expr: Expr::Var(var_id),
                    span: Span::new(0, 0),
                    ty: Type::Unknown,
                    custom_completion: None,
                },
                false,
            )
        } else {
            return (
                garbage(span),
                Some(ParseError::Mismatch(
                    "variable or subexpression".into(),
                    String::from_utf8_lossy(bytes).to_string(),
                    span,
                )),
            );
        };

        let (tail, err) = parse_cell_path(working_set, tokens, expect_dot, span);
        error = error.or(err);

        if !tail.is_empty() {
            (
                Expression {
                    expr: Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
                    ty: Type::Unknown,
                    span: full_cell_span,
                    custom_completion: None,
                },
                error,
            )
        } else {
            let ty = head.ty.clone();
            (
                Expression {
                    expr: Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
                    ty,
                    span: full_cell_span,
                    custom_completion: None,
                },
                error,
            )
        }
    } else {
        (garbage(span), error)
    }
}
