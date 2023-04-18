    pub fn parse_else_expr(&mut self) -> PResult<'a, P<Expr>> {
        if self.eat_keyword(keywords::If) {
            return self.parse_if_expr(ThinVec::new());
        } else {
            let blk = self.parse_block()?;
            return Ok(self.mk_expr(blk.span, ExprKind::Block(blk), ThinVec::new()));
        }
    }

    /// Parse a 'for' .. 'in' expression ('for' token already eaten)
    pub fn parse_for_expr(&mut self, opt_ident: Option<ast::SpannedIdent>,
                          span_lo: Span,
                          mut attrs: ThinVec<Attribute>) -> PResult<'a, P<Expr>> {
        // Parse: `for <src_pat> in <src_expr> <src_loop_block>`

        let pat = self.parse_pat()?;
        if !self.eat_keyword(keywords::In) {
            let in_span = self.prev_span.between(self.span);
            let mut err = self.sess.span_diagnostic
                .struct_span_err(in_span, "missing `in` in `for` loop");
            err.span_label(in_span, "expected `in` here");
            err.span_suggestion_short(in_span, "try adding `in` here", " in ".into());
            err.emit();
        }
        let expr = self.parse_expr_res(Restrictions::NO_STRUCT_LITERAL, None)?;
        let (iattrs, loop_block) = self.parse_inner_attrs_and_block()?;
        attrs.extend(iattrs);

        let hi = self.prev_span;
        Ok(self.mk_expr(span_lo.to(hi), ExprKind::ForLoop(pat, expr, loop_block, opt_ident), attrs))
    }
