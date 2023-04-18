    fn parse_stmt_internal(
        &mut self,
        start: BytePos,
        include_decl: bool,
        top_level: bool,
        decorators: Vec<Decorator>,
    ) -> PResult<'a, Stmt> {
        if top_level && is!("await") {
            let valid = self.target() >= JscTarget::Es2017 && self.syntax().top_level_await();

            if !valid {
                self.emit_err(self.input.cur_span(), SyntaxError::TopLevelAwait);
            }

            let expr = self.parse_await_expr()?;
            eat!(';');

            let span = span!(start);
            return Ok(Stmt::Expr(ExprStmt { span, expr }));
        }
        trace_cur!(parse_stmt_internal);

        if self.input.syntax().typescript() && is!("const") && peeked_is!("enum") {
            assert_and_bump!("const");
            assert_and_bump!("enum");
            return self
                .parse_ts_enum_decl(start, /* is_const */ true)
                .map(Decl::from)
                .map(Stmt::from);
        }

        if is_one_of!("break", "continue") {
            let is_break = is!("break");
            bump!();

            let label = if eat!(';') {
                None
            } else {
                let i = self.parse_label_ident().map(Some)?;
                expect!(';');
                i
            };

            let span = span!(start);
            if is_break {
                if label.is_some() && !self.state.labels.contains(&label.as_ref().unwrap().sym) {
                    self.emit_err(span, SyntaxError::TS1116);
                } else if !self.ctx().is_break_allowed {
                    self.emit_err(span, SyntaxError::TS1105);
                }
            } else {
                if !self.ctx().is_continue_allowed {
                    self.emit_err(span, SyntaxError::TS1115);
                } else if label.is_some()
                    && !self.state.labels.contains(&label.as_ref().unwrap().sym)
                {
                    self.emit_err(span, SyntaxError::TS1107);
                }
            }

            return Ok(if is_break {
                Stmt::Break(BreakStmt { span, label })
            } else {
                Stmt::Continue(ContinueStmt { span, label })
            });
        }

        if is!("debugger") {
            bump!();
            expect!(';');
            return Ok(Stmt::Debugger(DebuggerStmt { span: span!(start) }));
        }

        if is!("do") {
            return self.parse_do_stmt();
        }

        if is!("for") {
            return self.parse_for_stmt();
        }

        if is!("function") {
            if !include_decl {
                unexpected!()
            }

            return self.parse_fn_decl(decorators).map(Stmt::from);
        }

        if is!("class") {
            if !include_decl {
                unexpected!()
            }
            return self
                .parse_class_decl(start, start, decorators)
                .map(Stmt::from);
        }

        if is!("if") {
            return self.parse_if_stmt();
        }

        if is!("return") {
            return self.parse_return_stmt();
        }

        if is!("switch") {
            return self.parse_switch_stmt();
        }

        if is!("throw") {
            return self.parse_throw_stmt();
        }

        // Error
        if is!("catch") {
            let span = self.input.cur_span();
            self.emit_err(span, SyntaxError::TS1005);

            let _ = self.parse_catch_clause();
            let _ = self.parse_finally_block();

            return Ok(Stmt::Expr(ExprStmt {
                span,
                expr: Box::new(Expr::Invalid(Invalid { span })),
            }));
        }

        if is!("finally") {
            let span = self.input.cur_span();
            self.emit_err(span, SyntaxError::TS1005);

            let _ = self.parse_finally_block();

            return Ok(Stmt::Expr(ExprStmt {
                span,
                expr: Box::new(Expr::Invalid(Invalid { span })),
            }));
        }

        if is!("try") {
            return self.parse_try_stmt();
        }

        if is!("with") {
            return self.parse_with_stmt();
        }

        if is!("while") {
            return self.parse_while_stmt();
        }

        if is!("var") || (include_decl && is!("const")) {
            let v = self.parse_var_stmt(false)?;
            return Ok(Stmt::Decl(Decl::Var(v)));
        }

        // 'let' can start an identifier reference.
        if include_decl && is!("let") {
            let strict = self.ctx().strict;
            let is_keyword = match peek!() {
                Ok(t) => t.follows_keyword_let(strict),
                _ => false,
            };

            if is_keyword {
                let v = self.parse_var_stmt(false)?;
                return Ok(Stmt::Decl(Decl::Var(v)));
            }
        }

        if is!('{') {
            return self.parse_block(false).map(Stmt::Block);
        }

        if eat_exact!(';') {
            return Ok(Stmt::Empty(EmptyStmt { span: span!(start) }));
        }

        // Handle async function foo() {}
        if is!("async")
            && peeked_is!("function")
            && !self.input.has_linebreak_between_cur_and_peeked()
        {
            return self.parse_async_fn_decl(decorators).map(From::from);
        }

        // If the statement does not start with a statement keyword or a
        // brace, it's an ExpressionStatement or LabeledStatement. We
        // simply start parsing an expression, and afterwards, if the
        // next token is a colon and the expression was a simple
        // Identifier node, we switch to interpreting it as a label.
        let expr = self.include_in_expr(true).parse_expr()?;

        let expr = match *expr {
            Expr::Ident(ident) => {
                if eat!(':') {
                    return self.parse_labelled_stmt(ident);
                }
                Box::new(Expr::Ident(ident))
            }
            _ => self.verify_expr(expr)?,
        };
        if let Expr::Ident(ref ident) = *expr {
            if *ident.sym == js_word!("interface")
                && self.input.had_line_break_before_cur()
                && self.ctx().strict
            {
                self.emit_err(ident.span, SyntaxError::InvalidIdentInStrict);

                eat!(';');

                return Ok(Stmt::Expr(ExprStmt {
                    span: span!(start),
                    expr,
                }));
            }

            if self.input.syntax().typescript() {
                if let Some(decl) = self.parse_ts_expr_stmt(decorators, ident.clone())? {
                    return Ok(Stmt::Decl(decl));
                }
            }
        }

        if self.ctx().strict {
            match *expr {
                Expr::Ident(Ident { ref sym, span, .. }) => match *sym {
                    js_word!("enum") | js_word!("interface") => {
                        self.emit_err(span, SyntaxError::InvalidIdentInStrict);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if self.syntax().typescript() {
            match *expr {
                Expr::Ident(ref i) => match i.sym {
                    js_word!("public") | js_word!("static") | js_word!("abstract") => {
                        if eat!("interface") {
                            self.emit_err(i.span, SyntaxError::TS2427);
                            return self
                                .parse_ts_interface_decl(start)
                                .map(Decl::from)
                                .map(Stmt::from);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if eat!(';') {
            Ok(Stmt::Expr(ExprStmt {
                span: span!(start),
                expr,
            }))
        } else {
            match *cur!(false)? {
                Token::BinOp(..) => {
                    self.emit_err(self.input.cur_span(), SyntaxError::TS1005);
                    let expr = self.parse_bin_op_recursively(expr, 0)?;
                    return Ok(ExprStmt {
                        span: span!(start),
                        expr,
                    }
                    .into());
                }

                _ => {}
            }

            syntax_error!(SyntaxError::ExpectedSemiForExprStmt { expr: expr.span() });
        }
    }

    fn parse_if_stmt(&mut self) -> PResult<'a, Stmt> {
