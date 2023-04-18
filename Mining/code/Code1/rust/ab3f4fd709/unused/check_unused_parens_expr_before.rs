    fn check_unused_parens_expr(&self,
                                     cx: &EarlyContext<'_>,
                                     value: &ast::Expr,
                                     msg: &str,
                                     followed_by_block: bool,
                                     left_pos: Option<BytePos>,
                                     right_pos: Option<BytePos>) {
        match value.kind {
            ast::ExprKind::Paren(ref inner) => {
                if !Self::is_expr_parens_necessary(inner, followed_by_block) &&
                    value.attrs.is_empty() &&
                    !MultiSpan::from(value.span).primary_span()
                        .map_or(false, |span| span.from_expansion())
                {
                    let expr_text = if let Ok(snippet) = cx.sess().source_map()
                        .span_to_snippet(value.span) {
                            snippet
                        } else {
                            pprust::expr_to_string(value)
                        };
                    let keep_space = (
                        left_pos.map(|s| s >= value.span.lo()).unwrap_or(false),
                        right_pos.map(|s| s <= value.span.hi()).unwrap_or(false),
                    );
                    Self::remove_outer_parens(cx, value.span, &expr_text, msg, keep_space);
                }
            }
            ast::ExprKind::Let(_, ref expr) => {
                // FIXME(#60336): Properly handle `let true = (false && true)`
                // actually needing the parenthesis.
                self.check_unused_parens_expr(
                    cx, expr,
                    "`let` head expression",
                    followed_by_block,
                    None, None
                );
            }
            _ => {}
        }
    }

    fn check_unused_parens_pat(
        &self,
        cx: &EarlyContext<'_>,
        value: &ast::Pat,
        avoid_or: bool,
        avoid_mut: bool,
    ) {
        use ast::{PatKind, BindingMode::ByValue, Mutability::Mutable};

        if let PatKind::Paren(inner) = &value.kind {
            match inner.kind {
                // The lint visitor will visit each subpattern of `p`. We do not want to lint
                // any range pattern no matter where it occurs in the pattern. For something like
                // `&(a..=b)`, there is a recursive `check_pat` on `a` and `b`, but we will assume
                // that if there are unnecessary parens they serve a purpose of readability.
                PatKind::Range(..) => return,
                // Avoid `p0 | .. | pn` if we should.
                PatKind::Or(..) if avoid_or => return,
                // Avoid `mut x` and `mut x @ p` if we should:
                PatKind::Ident(ByValue(Mutable), ..) if avoid_mut => return,
                // Otherwise proceed with linting.
                _ => {}
            }

            let pattern_text = if let Ok(snippet) = cx.sess().source_map()
                .span_to_snippet(value.span) {
                    snippet
                } else {
                    pprust::pat_to_string(value)
                };
            Self::remove_outer_parens(cx, value.span, &pattern_text, "pattern", (false, false));
        }
    }
