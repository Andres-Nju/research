    fn parse_try_block(&mut self, span_lo: Span, mut attrs: ThinVec<Attribute>)
        -> PResult<'a, P<Expr>>
    {
        let (iattrs, body) = self.parse_inner_attrs_and_block()?;
        attrs.extend(iattrs);
        if self.eat_keyword(keywords::Catch) {
            let mut error = self.struct_span_err(self.prev_span,
                                                 "`try {} catch` is not a valid syntax");
            error.help("try using `match` on the result of the `try` block instead");
            Err(error)
        } else {
            Ok(self.mk_expr(span_lo.to(body.span), ExprKind::TryBlock(body), attrs))
        }
    }

    // `match` token already eaten
    fn parse_match_expr(&mut self, mut attrs: ThinVec<Attribute>) -> PResult<'a, P<Expr>> {
        let match_span = self.prev_span;
        let lo = self.prev_span;
        let discriminant = self.parse_expr_res(Restrictions::NO_STRUCT_LITERAL,
                                               None)?;
        if let Err(mut e) = self.expect(&token::OpenDelim(token::Brace)) {
            if self.token == token::Token::Semi {
                e.span_suggestion_short_with_applicability(
                    match_span,
                    "try removing this `match`",
                    String::new(),
                    Applicability::MaybeIncorrect // speculative
                );
            }
            return Err(e)
        }
        attrs.extend(self.parse_inner_attributes()?);

        let mut arms: Vec<Arm> = Vec::new();
        while self.token != token::CloseDelim(token::Brace) {
            match self.parse_arm() {
                Ok(arm) => arms.push(arm),
                Err(mut e) => {
                    // Recover by skipping to the end of the block.
                    e.emit();
                    self.recover_stmt();
                    let span = lo.to(self.span);
                    if self.token == token::CloseDelim(token::Brace) {
                        self.bump();
                    }
                    return Ok(self.mk_expr(span, ExprKind::Match(discriminant, arms), attrs));
                }
            }
        }
        let hi = self.span;
        self.bump();
        return Ok(self.mk_expr(lo.to(hi), ExprKind::Match(discriminant, arms), attrs));
    }
