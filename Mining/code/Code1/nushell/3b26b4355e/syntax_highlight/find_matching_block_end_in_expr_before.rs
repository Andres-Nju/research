fn find_matching_block_end_in_expr(
    line: &str,
    working_set: &StateWorkingSet,
    expression: &Expression,
    global_span_offset: usize,
    global_cursor_offset: usize,
) -> Option<usize> {
    macro_rules! find_in_expr_or_continue {
        ($inner_expr:ident) => {
            if let Some(pos) = find_matching_block_end_in_expr(
                line,
                working_set,
                $inner_expr,
                global_span_offset,
                global_cursor_offset,
            ) {
                return Some(pos);
            }
        };
    }

    if expression.span.contains(global_cursor_offset) {
        let expr_first = expression.span.start;
        let span_str = &line
            [expression.span.start - global_span_offset..expression.span.end - global_span_offset];
        let expr_last = span_str
            .chars()
            .last()
            .map(|c| expression.span.end - get_char_length(c))
            .unwrap_or(expression.span.start);

        return match &expression.expr {
            Expr::Bool(_) => None,
            Expr::Int(_) => None,
            Expr::Float(_) => None,
            Expr::Binary(_) => None,
            Expr::Range(..) => None,
            Expr::Var(_) => None,
            Expr::VarDecl(_) => None,
            Expr::ExternalCall(..) => None,
            Expr::Operator(_) => None,
            Expr::UnaryNot(_) => None,
            Expr::Keyword(..) => None,
            Expr::ValueWithUnit(..) => None,
            Expr::DateTime(_) => None,
            Expr::Filepath(_) => None,
            Expr::Directory(_) => None,
            Expr::GlobPattern(_) => None,
            Expr::String(_) => None,
            Expr::CellPath(_) => None,
            Expr::ImportPattern(_) => None,
            Expr::Overlay(_) => None,
            Expr::Signature(_) => None,
            Expr::Nothing => None,
            Expr::Garbage => None,

            Expr::Table(hdr, rows) => {
                if expr_last == global_cursor_offset {
                    // cursor is at table end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at table start
                    Some(expr_last)
                } else {
                    // cursor is inside table
                    for inner_expr in hdr {
                        find_in_expr_or_continue!(inner_expr);
                    }
                    for row in rows {
                        for inner_expr in row {
                            find_in_expr_or_continue!(inner_expr);
                        }
                    }
                    None
                }
            }

            Expr::Record(exprs) => {
                if expr_last == global_cursor_offset {
                    // cursor is at record end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at record start
                    Some(expr_last)
                } else {
                    // cursor is inside record
                    for (k, v) in exprs {
                        find_in_expr_or_continue!(k);
                        find_in_expr_or_continue!(v);
                    }
                    None
                }
            }

            Expr::Call(call) => {
                for arg in &call.arguments {
                    let opt_expr = match arg {
                        Argument::Named((_, _, opt_expr)) => opt_expr.as_ref(),
                        Argument::Positional(inner_expr) => Some(inner_expr),
                    };

                    if let Some(inner_expr) = opt_expr {
                        find_in_expr_or_continue!(inner_expr);
                    }
                }
                None
            }

            Expr::FullCellPath(b) => find_matching_block_end_in_expr(
                line,
                working_set,
                &b.head,
                global_span_offset,
                global_cursor_offset,
            ),

            Expr::BinaryOp(lhs, op, rhs) => {
                find_in_expr_or_continue!(lhs);
                find_in_expr_or_continue!(op);
                find_in_expr_or_continue!(rhs);
                None
            }

            Expr::Block(block_id)
            | Expr::RowCondition(block_id)
            | Expr::Subexpression(block_id) => {
                if expr_last == global_cursor_offset {
                    // cursor is at block end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at block start
                    Some(expr_last)
                } else {
                    // cursor is inside block
                    let nested_block = working_set.get_block(*block_id);
                    find_matching_block_end_in_block(
                        line,
                        working_set,
                        nested_block,
                        global_span_offset,
                        global_cursor_offset,
                    )
                }
            }

            Expr::StringInterpolation(inner_expr) => {
                for inner_expr in inner_expr {
                    find_in_expr_or_continue!(inner_expr);
                }
                None
            }

            Expr::List(inner_expr) => {
                if expr_last == global_cursor_offset {
                    // cursor is at list end
                    Some(expr_first)
                } else if expr_first == global_cursor_offset {
                    // cursor is at list start
                    Some(expr_last)
                } else {
                    // cursor is inside list
                    for inner_expr in inner_expr {
                        find_in_expr_or_continue!(inner_expr);
                    }
                    None
                }
            }
        };
    }
    None
}
