    fn parse_initializer(&mut self, skip_eq: bool) -> PResult<'a, Option<P<Expr>>> {
        if self.check(&token::Eq) {
            self.bump();
            Ok(Some(self.parse_expr()?))
        } else if skip_eq {
            Ok(Some(self.parse_expr()?))
        } else {
            Ok(None)
        }
    }

    /// Parse patterns, separated by '|' s
    fn parse_pats(&mut self) -> PResult<'a, Vec<P<Pat>>> {
        let mut pats = Vec::new();
        loop {
            pats.push(self.parse_pat()?);

            if self.token == token::OrOr {
                let mut err = self.struct_span_err(self.span, "unexpected token `||` after pattern");
                err.span_suggestion(self.span, "use a single `|` to specify multiple patterns", "|".to_owned());
                err.emit();
                self.bump();
            } else if self.check(&token::BinOp(token::Or)) {
                self.bump();
            } else {
                return Ok(pats);
            }
        };
    }
