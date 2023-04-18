    fn parse_assoc_op_cast(&mut self, lhs: P<Expr>, lhs_span: Span,
                           expr_kind: fn(P<Expr>, P<Ty>) -> ExprKind)
                           -> PResult<'a, P<Expr>> {
        let mk_expr = |this: &mut Self, rhs: P<Ty>| {
            this.mk_expr(lhs_span.to(rhs.span), expr_kind(lhs, rhs), ThinVec::new())
        };

        // Save the state of the parser before parsing type normally, in case there is a
        // LessThan comparison after this cast.
        let parser_snapshot_before_type = self.clone();
        match self.parse_ty_no_plus() {
            Ok(rhs) => {
                Ok(mk_expr(self, rhs))
            }
            Err(mut type_err) => {
                // Rewind to before attempting to parse the type with generics, to recover
                // from situations like `x as usize < y` in which we first tried to parse
                // `usize < y` as a type with generic arguments.
                let parser_snapshot_after_type = self.clone();
                mem::replace(self, parser_snapshot_before_type);

                match self.parse_path(PathStyle::Expr) {
                    Ok(path) => {
                        // Successfully parsed the type path leaving a `<` yet to parse.
                        type_err.cancel();

                        // Report non-fatal diagnostics, keep `x as usize` as an expression
                        // in AST and continue parsing.
                        let msg = format!("`<` is interpreted as a start of generic \
                                           arguments for `{}`, not a comparison", path);
                        let mut err = self.sess.span_diagnostic.struct_span_err(self.span, &msg);
                        err.span_label(self.look_ahead_span(1).to(parser_snapshot_after_type.span),
                                       "interpreted as generic arguments");
                        err.span_label(self.span, "not interpreted as comparison");

                        let expr = mk_expr(self, P(Ty {
                            span: path.span,
                            node: TyKind::Path(None, path),
                            id: ast::DUMMY_NODE_ID
                        }));

                        let expr_str = self.sess.codemap().span_to_snippet(expr.span)
                                                .unwrap_or(pprust::expr_to_string(&expr));
                        err.span_suggestion(expr.span,
                                            "try comparing the casted value",
                                            format!("({})", expr_str));
                        err.emit();

                        Ok(expr)
                    }
                    Err(mut path_err) => {
                        // Couldn't parse as a path, return original error and parser state.
                        path_err.cancel();
                        mem::replace(self, parser_snapshot_after_type);
                        Err(type_err)
                    }
                }
            }
        }
    }

    /// Produce an error if comparison operators are chained (RFC #558).
    /// We only need to check lhs, not rhs, because all comparison ops
    /// have same precedence and are left-associative
    fn check_no_chained_comparison(&mut self, lhs: &Expr, outer_op: &AssocOp) {
        debug_assert!(outer_op.is_comparison(),
                      "check_no_chained_comparison: {:?} is not comparison",
                      outer_op);
        match lhs.node {
            ExprKind::Binary(op, _, _) if op.node.is_comparison() => {
                // respan to include both operators
                let op_span = op.span.to(self.span);
                let mut err = self.diagnostic().struct_span_err(op_span,
                    "chained comparison operators require parentheses");
                if op.node == BinOpKind::Lt &&
                    *outer_op == AssocOp::Less ||  // Include `<` to provide this recommendation
                    *outer_op == AssocOp::Greater  // even in a case like the following:
                {                                  //     Foo<Bar<Baz<Qux, ()>>>
                    err.help(
                        "use `::<...>` instead of `<...>` if you meant to specify type arguments");
                }
                err.emit();
            }
            _ => {}
        }
    }
