    fn parse_struct_expr(&mut self, lo: Span, pth: ast::Path, mut attrs: ThinVec<Attribute>)
                         -> PResult<'a, P<Expr>> {
        let struct_sp = lo.to(self.prev_span);
        self.bump();
        let mut fields = Vec::new();
        let mut base = None;

        attrs.extend(self.parse_inner_attributes()?);

        while self.token != token::CloseDelim(token::Brace) {
            if self.eat(&token::DotDot) {
                let exp_span = self.prev_span;
                match self.parse_expr() {
                    Ok(e) => {
                        base = Some(e);
                    }
                    Err(mut e) => {
                        e.emit();
                        self.recover_stmt();
                    }
                }
                if self.token == token::Comma {
                    let mut err = self.sess.span_diagnostic.mut_span_err(
                        exp_span.to(self.prev_span),
                        "cannot use a comma after the base struct",
                    );
                    err.span_suggestion_short_with_applicability(
                        self.span,
                        "remove this comma",
                        "".to_owned(),
                        Applicability::MachineApplicable
                    );
                    err.note("the base struct must always be the last field");
                    err.emit();
                    self.recover_stmt();
                }
                break;
            }

            match self.parse_field() {
                Ok(f) => fields.push(f),
                Err(mut e) => {
                    e.span_label(struct_sp, "while parsing this struct");
                    e.emit();

                    // If the next token is a comma, then try to parse
                    // what comes next as additional fields, rather than
                    // bailing out until next `}`.
                    if self.token != token::Comma {
                        self.recover_stmt();
                        break;
                    }
                }
            }

            match self.expect_one_of(&[token::Comma],
                                     &[token::CloseDelim(token::Brace)]) {
                Ok(()) => {}
                Err(mut e) => {
                    e.emit();
                    self.recover_stmt();
                    break;
                }
            }
        }

        let span = lo.to(self.span);
        self.expect(&token::CloseDelim(token::Brace))?;
        return Ok(self.mk_expr(span, ExprKind::Struct(pth, fields, base), attrs));
    }

    fn parse_or_use_outer_attributes(&mut self,
                                     already_parsed_attrs: Option<ThinVec<Attribute>>)
                                     -> PResult<'a, ThinVec<Attribute>> {
        if let Some(attrs) = already_parsed_attrs {
            Ok(attrs)
        } else {
            self.parse_outer_attributes().map(|a| a.into())
        }
    }
