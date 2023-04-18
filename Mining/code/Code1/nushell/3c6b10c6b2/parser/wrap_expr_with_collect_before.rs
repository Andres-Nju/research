fn wrap_expr_with_collect(working_set: &mut StateWorkingSet, expr: &Expression) -> Expression {
    let span = expr.span;

    if let Some(decl_id) = working_set.find_decl(b"collect", &Type::Any) {
        let mut output = vec![];

        let var_id = working_set.next_var_id();
        let mut signature = Signature::new("");
        signature.required_positional.push(PositionalArg {
            var_id: Some(var_id),
            name: "$in".into(),
            desc: String::new(),
            shape: SyntaxShape::Any,
            default_value: None,
        });

        let mut expr = expr.clone();
        expr.replace_in_variable(working_set, var_id);

        let block = Block {
            pipelines: vec![Pipeline::from_vec(vec![expr])],
            signature: Box::new(signature),
            ..Default::default()
        };

        let block_id = working_set.add_block(block);

        output.push(Argument::Positional(Expression {
            expr: Expr::Closure(block_id),
            span,
            ty: Type::Any,
            custom_completion: None,
        }));

        output.push(Argument::Named((
            Spanned {
                item: "keep-env".to_string(),
                span: Span::new(0, 0),
            },
            None,
            None,
        )));

        // The containing, synthetic call to `collect`.
        // We don't want to have a real span as it will confuse flattening
        // The args are where we'll get the real info
        Expression {
            expr: Expr::Call(Box::new(Call {
                head: Span::new(0, 0),
                arguments: output,
                decl_id,
                redirect_stdout: true,
                redirect_stderr: false,
                parser_info: vec![],
            })),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    } else {
        Expression::garbage(span)
    }
}
