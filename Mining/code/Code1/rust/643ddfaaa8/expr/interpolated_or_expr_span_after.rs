    fn interpolated_or_expr_span(
        &self,
        expr: PResult<'a, P<Expr>>,
    ) -> PResult<'a, (Span, P<Expr>)> {
        expr.map(|e| {
            if self.prev_token_kind == PrevTokenKind::Interpolated {
                (self.prev_span, e)
            } else {
                (e.span, e)
            }
        })
    }

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
                        let (op_noun, op_verb) = match self.token.kind {
                            token::Lt => ("comparison", "comparing"),
                            token::BinOp(token::Shl) => ("shift", "shifting"),
                            _ => {
                                // We can end up here even without `<` being the next token, for
                                // example because `parse_ty_no_plus` returns `Err` on keywords,
                                // but `parse_path` returns `Ok` on them due to error recovery.
                                // Return original error and parser state.
                                mem::replace(self, parser_snapshot_after_type);
                                return Err(type_err);
                            }
                        };

                        // Successfully parsed the type path leaving a `<` yet to parse.
                        type_err.cancel();

                        // Report non-fatal diagnostics, keep `x as usize` as an expression
                        // in AST and continue parsing.
                        let msg = format!("`<` is interpreted as a start of generic \
                                           arguments for `{}`, not a {}", path, op_noun);
                        let span_after_type = parser_snapshot_after_type.token.span;
                        let expr = mk_expr(self, P(Ty {
                            span: path.span,
                            node: TyKind::Path(None, path),
                            id: ast::DUMMY_NODE_ID
                        }));

                        let expr_str = self.span_to_snippet(expr.span)
                            .unwrap_or_else(|_| pprust::expr_to_string(&expr));

                        self.struct_span_err(self.token.span, &msg)
                            .span_label(
                                self.look_ahead(1, |t| t.span).to(span_after_type),
                                "interpreted as generic arguments"
                            )
                            .span_label(self.token.span, format!("not interpreted as {}", op_noun))
                            .span_suggestion(
                                expr.span,
                                &format!("try {} the cast value", op_verb),
                                format!("({})", expr_str),
                                Applicability::MachineApplicable
                            )
                            .emit();

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

    /// Parses `a.b` or `a(13)` or `a[4]` or just `a`.
    fn parse_dot_or_call_expr(
        &mut self,
        already_parsed_attrs: Option<ThinVec<Attribute>>,
    ) -> PResult<'a, P<Expr>> {
        let attrs = self.parse_or_use_outer_attributes(already_parsed_attrs)?;

        let b = self.parse_bottom_expr();
        let (span, b) = self.interpolated_or_expr_span(b)?;
        self.parse_dot_or_call_expr_with(b, span, attrs)
    }

    pub(super) fn parse_dot_or_call_expr_with(
        &mut self,
        e0: P<Expr>,
        lo: Span,
        mut attrs: ThinVec<Attribute>,
    ) -> PResult<'a, P<Expr>> {
        // Stitch the list of outer attributes onto the return value.
        // A little bit ugly, but the best way given the current code
        // structure
        self.parse_dot_or_call_expr_with_(e0, lo).map(|expr|
            expr.map(|mut expr| {
                attrs.extend::<Vec<_>>(expr.attrs.into());
                expr.attrs = attrs;
                match expr.node {
                    ExprKind::If(..) if !expr.attrs.is_empty() => {
                        // Just point to the first attribute in there...
                        let span = expr.attrs[0].span;
                        self.span_err(span, "attributes are not yet allowed on `if` expressions");
                    }
                    _ => {}
                }
                expr
            })
        )
    }

    fn parse_dot_or_call_expr_with_(&mut self, e0: P<Expr>, lo: Span) -> PResult<'a, P<Expr>> {
        let mut e = e0;
        let mut hi;
        loop {
            // expr?
            while self.eat(&token::Question) {
                let hi = self.prev_span;
                e = self.mk_expr(lo.to(hi), ExprKind::Try(e), ThinVec::new());
            }

            // expr.f
            if self.eat(&token::Dot) {
                match self.token.kind {
                    token::Ident(..) => {
                        e = self.parse_dot_suffix(e, lo)?;
                    }
                    token::Literal(token::Lit { kind: token::Integer, symbol, suffix }) => {
                        let span = self.token.span;
                        self.bump();
                        let field = ExprKind::Field(e, Ident::new(symbol, span));
                        e = self.mk_expr(lo.to(span), field, ThinVec::new());

                        self.expect_no_suffix(span, "a tuple index", suffix);
                    }
                    token::Literal(token::Lit { kind: token::Float, symbol, .. }) => {
                      self.bump();
                      let fstr = symbol.as_str();
                      let msg = format!("unexpected token: `{}`", symbol);
                      let mut err = self.diagnostic().struct_span_err(self.prev_span, &msg);
                      err.span_label(self.prev_span, "unexpected token");
                      if fstr.chars().all(|x| "0123456789.".contains(x)) {
                          let float = match fstr.parse::<f64>().ok() {
                              Some(f) => f,
                              None => continue,
                          };
                          let sugg = pprust::to_string(|s| {
                              s.popen();
                              s.print_expr(&e);
                              s.s.word( ".");
                              s.print_usize(float.trunc() as usize);
                              s.pclose();
                              s.s.word(".");
                              s.s.word(fstr.splitn(2, ".").last().unwrap().to_string())
                          });
                          err.span_suggestion(
                              lo.to(self.prev_span),
                              "try parenthesizing the first index",
                              sugg,
                              Applicability::MachineApplicable
                          );
                      }
                      return Err(err);

                    }
                    _ => {
                        // FIXME Could factor this out into non_fatal_unexpected or something.
                        let actual = self.this_token_to_string();
                        self.span_err(self.token.span, &format!("unexpected token: `{}`", actual));
                    }
                }
                continue;
            }
            if self.expr_is_complete(&e) { break; }
            match self.token.kind {
                // expr(...)
                token::OpenDelim(token::Paren) => {
                    let seq = self.parse_paren_expr_seq().map(|es| {
                        let nd = self.mk_call(e, es);
                        let hi = self.prev_span;
                        self.mk_expr(lo.to(hi), nd, ThinVec::new())
                    });
                    e = self.recover_seq_parse_error(token::Paren, lo, seq);
                }

                // expr[...]
                // Could be either an index expression or a slicing expression.
                token::OpenDelim(token::Bracket) => {
                    self.bump();
                    let ix = self.parse_expr()?;
                    hi = self.token.span;
                    self.expect(&token::CloseDelim(token::Bracket))?;
                    let index = self.mk_index(e, ix);
                    e = self.mk_expr(lo.to(hi), index, ThinVec::new())
                }
                _ => return Ok(e)
            }
        }
        return Ok(e);
    }

    /// Assuming we have just parsed `.`, continue parsing into an expression.
    fn parse_dot_suffix(&mut self, self_arg: P<Expr>, lo: Span) -> PResult<'a, P<Expr>> {
        if self.token.span.rust_2018() && self.eat_keyword(kw::Await) {
            return self.mk_await_expr(self_arg, lo);
        }

        let segment = self.parse_path_segment(PathStyle::Expr)?;
        self.check_trailing_angle_brackets(&segment, token::OpenDelim(token::Paren));

        Ok(match self.token.kind {
            token::OpenDelim(token::Paren) => {
                // Method call `expr.f()`
                let mut args = self.parse_paren_expr_seq()?;
                args.insert(0, self_arg);

                let span = lo.to(self.prev_span);
                self.mk_expr(span, ExprKind::MethodCall(segment, args), ThinVec::new())
            }
            _ => {
                // Field access `expr.f`
                if let Some(args) = segment.args {
                    self.span_err(args.span(),
                                  "field expressions may not have generic arguments");
                }

                let span = lo.to(self.prev_span);
                self.mk_expr(span, ExprKind::Field(self_arg, segment.ident), ThinVec::new())
            }
        })
    }


    /// At the bottom (top?) of the precedence hierarchy,
    /// Parses things like parenthesized exprs, macros, `return`, etc.
    ///
    /// N.B., this does not parse outer attributes, and is private because it only works
    /// correctly if called from `parse_dot_or_call_expr()`.
    fn parse_bottom_expr(&mut self) -> PResult<'a, P<Expr>> {
        maybe_recover_from_interpolated_ty_qpath!(self, true);
        maybe_whole_expr!(self);

        // Outer attributes are already parsed and will be
        // added to the return value after the fact.
        //
        // Therefore, prevent sub-parser from parsing
        // attributes by giving them a empty "already parsed" list.
        let mut attrs = ThinVec::new();

        let lo = self.token.span;
        let mut hi = self.token.span;

        let ex: ExprKind;

        macro_rules! parse_lit {
            () => {
                match self.parse_lit() {
                    Ok(literal) => {
                        hi = self.prev_span;
                        ex = ExprKind::Lit(literal);
                    }
                    Err(mut err) => {
                        self.cancel(&mut err);
                        return Err(self.expected_expression_found());
                    }
                }
            }
        }

        // Note: when adding new syntax here, don't forget to adjust TokenKind::can_begin_expr().
        match self.token.kind {
            // This match arm is a special-case of the `_` match arm below and
            // could be removed without changing functionality, but it's faster
            // to have it here, especially for programs with large constants.
            token::Literal(_) => {
                parse_lit!()
            }
            token::OpenDelim(token::Paren) => {
                self.bump();

                attrs.extend(self.parse_inner_attributes()?);

                // (e) is parenthesized e
                // (e,) is a tuple with only one field, e
                let mut es = vec![];
                let mut trailing_comma = false;
                let mut recovered = false;
                while self.token != token::CloseDelim(token::Paren) {
                    es.push(match self.parse_expr() {
                        Ok(es) => es,
                        Err(mut err) => {
                            // recover from parse error in tuple list
                            match self.token.kind {
                                token::Ident(name, false)
                                if name == kw::Underscore && self.look_ahead(1, |t| {
                                    t == &token::Comma
                                }) => {
                                    // Special-case handling of `Foo<(_, _, _)>`
                                    err.emit();
                                    let sp = self.token.span;
                                    self.bump();
                                    self.mk_expr(sp, ExprKind::Err, ThinVec::new())
                                }
                                _ => return Ok(
                                    self.recover_seq_parse_error(token::Paren, lo, Err(err)),
                                ),
                            }
                        }
                    });
                    recovered = self.expect_one_of(
                        &[],
                        &[token::Comma, token::CloseDelim(token::Paren)],
                    )?;
                    if self.eat(&token::Comma) {
                        trailing_comma = true;
                    } else {
                        trailing_comma = false;
                        break;
                    }
                }
                if !recovered {
                    self.bump();
                }

                hi = self.prev_span;
                ex = if es.len() == 1 && !trailing_comma {
                    ExprKind::Paren(es.into_iter().nth(0).unwrap())
                } else {
                    ExprKind::Tup(es)
                };
            }
            token::OpenDelim(token::Brace) => {
                return self.parse_block_expr(None, lo, BlockCheckMode::Default, attrs);
            }
            token::BinOp(token::Or) | token::OrOr => {
                return self.parse_lambda_expr(attrs);
            }
            token::OpenDelim(token::Bracket) => {
                self.bump();

                attrs.extend(self.parse_inner_attributes()?);

                if self.eat(&token::CloseDelim(token::Bracket)) {
                    // Empty vector.
                    ex = ExprKind::Array(Vec::new());
                } else {
                    // Nonempty vector.
                    let first_expr = self.parse_expr()?;
                    if self.eat(&token::Semi) {
                        // Repeating array syntax: [ 0; 512 ]
                        let count = AnonConst {
                            id: ast::DUMMY_NODE_ID,
                            value: self.parse_expr()?,
                        };
                        self.expect(&token::CloseDelim(token::Bracket))?;
                        ex = ExprKind::Repeat(first_expr, count);
                    } else if self.eat(&token::Comma) {
                        // Vector with two or more elements.
                        let remaining_exprs = self.parse_seq_to_end(
                            &token::CloseDelim(token::Bracket),
                            SeqSep::trailing_allowed(token::Comma),
                            |p| Ok(p.parse_expr()?)
                        )?;
                        let mut exprs = vec![first_expr];
                        exprs.extend(remaining_exprs);
                        ex = ExprKind::Array(exprs);
                    } else {
                        // Vector with one element.
                        self.expect(&token::CloseDelim(token::Bracket))?;
                        ex = ExprKind::Array(vec![first_expr]);
                    }
                }
                hi = self.prev_span;
            }
            _ => {
                if self.eat_lt() {
                    let (qself, path) = self.parse_qpath(PathStyle::Expr)?;
                    hi = path.span;
                    return Ok(self.mk_expr(lo.to(hi), ExprKind::Path(Some(qself), path), attrs));
                }
                if self.check_keyword(kw::Move) || self.check_keyword(kw::Static) {
                    return self.parse_lambda_expr(attrs);
                }
                if self.eat_keyword(kw::If) {
                    return self.parse_if_expr(attrs);
                }
                if self.eat_keyword(kw::For) {
                    let lo = self.prev_span;
                    return self.parse_for_expr(None, lo, attrs);
                }
                if self.eat_keyword(kw::While) {
                    let lo = self.prev_span;
                    return self.parse_while_expr(None, lo, attrs);
                }
                if let Some(label) = self.eat_label() {
                    let lo = label.ident.span;
                    self.expect(&token::Colon)?;
                    if self.eat_keyword(kw::While) {
                        return self.parse_while_expr(Some(label), lo, attrs)
                    }
                    if self.eat_keyword(kw::For) {
                        return self.parse_for_expr(Some(label), lo, attrs)
                    }
                    if self.eat_keyword(kw::Loop) {
                        return self.parse_loop_expr(Some(label), lo, attrs)
                    }
                    if self.token == token::OpenDelim(token::Brace) {
                        return self.parse_block_expr(Some(label),
                                                     lo,
                                                     BlockCheckMode::Default,
                                                     attrs);
                    }
                    let msg = "expected `while`, `for`, `loop` or `{` after a label";
                    let mut err = self.fatal(msg);
                    err.span_label(self.token.span, msg);
                    return Err(err);
                }
                if self.eat_keyword(kw::Loop) {
                    let lo = self.prev_span;
                    return self.parse_loop_expr(None, lo, attrs);
                }
                if self.eat_keyword(kw::Continue) {
                    let label = self.eat_label();
                    let ex = ExprKind::Continue(label);
                    let hi = self.prev_span;
                    return Ok(self.mk_expr(lo.to(hi), ex, attrs));
                }
                if self.eat_keyword(kw::Match) {
                    let match_sp = self.prev_span;
                    return self.parse_match_expr(attrs).map_err(|mut err| {
                        err.span_label(match_sp, "while parsing this match expression");
                        err
                    });
                }
                if self.eat_keyword(kw::Unsafe) {
                    return self.parse_block_expr(
                        None,
                        lo,
                        BlockCheckMode::Unsafe(ast::UserProvided),
                        attrs);
                }
                if self.is_do_catch_block() {
                    let mut db = self.fatal("found removed `do catch` syntax");
                    db.help("Following RFC #2388, the new non-placeholder syntax is `try`");
                    return Err(db);
                }
                if self.is_try_block() {
                    let lo = self.token.span;
                    assert!(self.eat_keyword(kw::Try));
                    return self.parse_try_block(lo, attrs);
                }

                // Span::rust_2018() is somewhat expensive; don't get it repeatedly.
                let is_span_rust_2018 = self.token.span.rust_2018();
                if is_span_rust_2018 && self.check_keyword(kw::Async) {
                    return if self.is_async_block() { // check for `async {` and `async move {`
                        self.parse_async_block(attrs)
                    } else {
                        self.parse_lambda_expr(attrs)
                    };
                }
                if self.eat_keyword(kw::Return) {
                    if self.token.can_begin_expr() {
                        let e = self.parse_expr()?;
                        hi = e.span;
                        ex = ExprKind::Ret(Some(e));
                    } else {
                        ex = ExprKind::Ret(None);
                    }
                } else if self.eat_keyword(kw::Break) {
                    let label = self.eat_label();
                    let e = if self.token.can_begin_expr()
                               && !(self.token == token::OpenDelim(token::Brace)
                                    && self.restrictions.contains(
                                           Restrictions::NO_STRUCT_LITERAL)) {
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    ex = ExprKind::Break(label, e);
                    hi = self.prev_span;
                } else if self.eat_keyword(kw::Yield) {
                    if self.token.can_begin_expr() {
                        let e = self.parse_expr()?;
                        hi = e.span;
                        ex = ExprKind::Yield(Some(e));
                    } else {
                        ex = ExprKind::Yield(None);
                    }
                } else if self.eat_keyword(kw::Let) {
                    return self.parse_let_expr(attrs);
                } else if is_span_rust_2018 && self.eat_keyword(kw::Await) {
                    let (await_hi, e_kind) = self.parse_incorrect_await_syntax(lo, self.prev_span)?;
                    hi = await_hi;
                    ex = e_kind;
                } else if self.token.is_path_start() {
                    let path = self.parse_path(PathStyle::Expr)?;

                    // `!`, as an operator, is prefix, so we know this isn't that
                    if self.eat(&token::Not) {
                        // MACRO INVOCATION expression
                        let (delim, tts) = self.expect_delimited_token_tree()?;
                        hi = self.prev_span;
                        ex = ExprKind::Mac(respan(lo.to(hi), Mac_ {
                            path,
                            tts,
                            delim,
                            prior_type_ascription: self.last_type_ascription,
                        }));
                    } else if self.check(&token::OpenDelim(token::Brace)) {
                        if let Some(expr) = self.maybe_parse_struct_expr(lo, &path, &attrs) {
                            return expr;
                        } else {
                            hi = path.span;
                            ex = ExprKind::Path(None, path);
                        }
                    } else {
                        hi = path.span;
                        ex = ExprKind::Path(None, path);
                    }
                } else {
                    if !self.unclosed_delims.is_empty() && self.check(&token::Semi) {
                        // Don't complain about bare semicolons after unclosed braces
                        // recovery in order to keep the error count down. Fixing the
                        // delimiters will possibly also fix the bare semicolon found in
                        // expression context. For example, silence the following error:
                        // ```
                        // error: expected expression, found `;`
                        //  --> file.rs:2:13
                        //   |
                        // 2 |     foo(bar(;
                        //   |             ^ expected expression
                        // ```
                        self.bump();
                        return Ok(self.mk_expr(self.token.span, ExprKind::Err, ThinVec::new()));
                    }
                    parse_lit!()
                }
            }
        }

        let expr = self.mk_expr(lo.to(hi), ex, attrs);
        self.maybe_recover_from_bad_qpath(expr, true)
    }

    /// Matches `'-' lit | lit` (cf. `ast_validation::AstValidator::check_expr_within_pat`).
    crate fn parse_literal_maybe_minus(&mut self) -> PResult<'a, P<Expr>> {
        maybe_whole_expr!(self);

        let minus_lo = self.token.span;
        let minus_present = self.eat(&token::BinOp(token::Minus));
        let lo = self.token.span;
        let literal = self.parse_lit()?;
        let hi = self.prev_span;
        let expr = self.mk_expr(lo.to(hi), ExprKind::Lit(literal), ThinVec::new());

        if minus_present {
            let minus_hi = self.prev_span;
            let unary = self.mk_unary(UnOp::Neg, expr);
            Ok(self.mk_expr(minus_lo.to(minus_hi), unary, ThinVec::new()))
        } else {
            Ok(expr)
        }
    }

    /// Parses a block or unsafe block.
    crate fn parse_block_expr(
        &mut self,
        opt_label: Option<Label>,
        lo: Span,
        blk_mode: BlockCheckMode,
        outer_attrs: ThinVec<Attribute>,
    ) -> PResult<'a, P<Expr>> {
        self.expect(&token::OpenDelim(token::Brace))?;

        let mut attrs = outer_attrs;
        attrs.extend(self.parse_inner_attributes()?);

        let blk = self.parse_block_tail(lo, blk_mode)?;
        return Ok(self.mk_expr(blk.span, ExprKind::Block(blk, opt_label), attrs));
    }

    /// Parses `move |args| expr`.
    fn parse_lambda_expr(&mut self, attrs: ThinVec<Attribute>) -> PResult<'a, P<Expr>> {
        let lo = self.token.span;

        let movability = if self.eat_keyword(kw::Static) {
            Movability::Static
        } else {
            Movability::Movable
        };

        let asyncness = if self.token.span.rust_2018() {
            self.parse_asyncness()
        } else {
            IsAsync::NotAsync
        };
        if asyncness.is_async() {
            // Feature gate `async ||` closures.
            self.sess.async_closure_spans.borrow_mut().push(self.prev_span);
        }

        let capture_clause = self.parse_capture_clause();
        let decl = self.parse_fn_block_decl()?;
        let decl_hi = self.prev_span;
        let body = match decl.output {
            FunctionRetTy::Default(_) => {
                let restrictions = self.restrictions - Restrictions::STMT_EXPR;
                self.parse_expr_res(restrictions, None)?
            },
            _ => {
                // If an explicit return type is given, require a
                // block to appear (RFC 968).
                let body_lo = self.token.span;
                self.parse_block_expr(None, body_lo, BlockCheckMode::Default, ThinVec::new())?
            }
        };

        Ok(self.mk_expr(
            lo.to(body.span),
            ExprKind::Closure(capture_clause, asyncness, movability, decl, body, lo.to(decl_hi)),
            attrs))
    }

    /// Parse an optional `move` prefix to a closure lke construct.
    fn parse_capture_clause(&mut self) -> CaptureBy {
        if self.eat_keyword(kw::Move) {
            CaptureBy::Value
        } else {
            CaptureBy::Ref
        }
    }

    /// Parses the `|arg, arg|` header of a closure.
    fn parse_fn_block_decl(&mut self) -> PResult<'a, P<FnDecl>> {
        let inputs_captures = {
            if self.eat(&token::OrOr) {
                Vec::new()
            } else {
                self.expect(&token::BinOp(token::Or))?;
                let args = self.parse_seq_to_before_tokens(
                    &[&token::BinOp(token::Or), &token::OrOr],
                    SeqSep::trailing_allowed(token::Comma),
                    TokenExpectType::NoExpect,
                    |p| p.parse_fn_block_arg()
                )?.0;
                self.expect_or()?;
                args
            }
        };
        let output = self.parse_ret_ty(true)?;

        Ok(P(FnDecl {
            inputs: inputs_captures,
            output,
            c_variadic: false
        }))
    }

    /// Parses an argument in a lambda header (e.g., `|arg, arg|`).
    fn parse_fn_block_arg(&mut self) -> PResult<'a, Arg> {
        let lo = self.token.span;
        let attrs = self.parse_arg_attributes()?;
        let pat = self.parse_pat(Some("argument name"))?;
        let t = if self.eat(&token::Colon) {
            self.parse_ty()?
        } else {
            P(Ty {
                id: ast::DUMMY_NODE_ID,
                node: TyKind::Infer,
                span: self.prev_span,
            })
        };
        let span = lo.to(self.token.span);
        Ok(Arg {
            attrs: attrs.into(),
            ty: t,
            pat,
            span,
            id: ast::DUMMY_NODE_ID
        })
    }

    /// Parses an `if` expression (`if` token already eaten).
    fn parse_if_expr(&mut self, attrs: ThinVec<Attribute>) -> PResult<'a, P<Expr>> {
        let lo = self.prev_span;
        let cond = self.parse_cond_expr()?;

        // Verify that the parsed `if` condition makes sense as a condition. If it is a block, then
        // verify that the last statement is either an implicit return (no `;`) or an explicit
        // return. This won't catch blocks with an explicit `return`, but that would be caught by
        // the dead code lint.
        if self.eat_keyword(kw::Else) || !cond.returns() {
            let sp = self.sess.source_map().next_point(lo);
            let mut err = self.diagnostic()
                .struct_span_err(sp, "missing condition for `if` expression");
            err.span_label(sp, "expected if condition here");
            return Err(err)
        }
        let not_block = self.token != token::OpenDelim(token::Brace);
        let thn = self.parse_block().map_err(|mut err| {
            if not_block {
                err.span_label(lo, "this `if` statement has a condition, but no block");
            }
            err
        })?;
        let mut els: Option<P<Expr>> = None;
        let mut hi = thn.span;
        if self.eat_keyword(kw::Else) {
            let elexpr = self.parse_else_expr()?;
            hi = elexpr.span;
            els = Some(elexpr);
        }
        Ok(self.mk_expr(lo.to(hi), ExprKind::If(cond, thn, els), attrs))
    }

    /// Parse the condition of a `if`- or `while`-expression
    fn parse_cond_expr(&mut self) -> PResult<'a, P<Expr>> {
        let cond = self.parse_expr_res(Restrictions::NO_STRUCT_LITERAL, None)?;

        if let ExprKind::Let(..) = cond.node {
            // Remove the last feature gating of a `let` expression since it's stable.
            let last = self.sess.let_chains_spans.borrow_mut().pop();
            debug_assert_eq!(cond.span, last.unwrap());
        }

        Ok(cond)
    }

    /// Parses a `let $pats = $expr` pseudo-expression.
    /// The `let` token has already been eaten.
    fn parse_let_expr(&mut self, attrs: ThinVec<Attribute>) -> PResult<'a, P<Expr>> {
        let lo = self.prev_span;
        let pats = self.parse_pats()?;
        self.expect(&token::Eq)?;
        let expr = self.with_res(
            Restrictions::NO_STRUCT_LITERAL,
            |this| this.parse_assoc_expr_with(1 + prec_let_scrutinee_needs_par(), None.into())
        )?;
        let span = lo.to(expr.span);
        self.sess.let_chains_spans.borrow_mut().push(span);
        Ok(self.mk_expr(span, ExprKind::Let(pats, expr), attrs))
    }

    /// `else` token already eaten
    fn parse_else_expr(&mut self) -> PResult<'a, P<Expr>> {
        if self.eat_keyword(kw::If) {
            return self.parse_if_expr(ThinVec::new());
        } else {
            let blk = self.parse_block()?;
            return Ok(self.mk_expr(blk.span, ExprKind::Block(blk, None), ThinVec::new()));
        }
    }

    /// Parse a 'for' .. 'in' expression ('for' token already eaten)
    fn parse_for_expr(
        &mut self,
        opt_label: Option<Label>,
        span_lo: Span,
        mut attrs: ThinVec<Attribute>
    ) -> PResult<'a, P<Expr>> {
        // Parse: `for <src_pat> in <src_expr> <src_loop_block>`

        // Record whether we are about to parse `for (`.
        // This is used below for recovery in case of `for ( $stuff ) $block`
        // in which case we will suggest `for $stuff $block`.
        let begin_paren = match self.token.kind {
            token::OpenDelim(token::Paren) => Some(self.token.span),
            _ => None,
        };

        let pat = self.parse_top_level_pat()?;
        if !self.eat_keyword(kw::In) {
            let in_span = self.prev_span.between(self.token.span);
            self.struct_span_err(in_span, "missing `in` in `for` loop")
                .span_suggestion_short(
                    in_span,
                    "try adding `in` here", " in ".into(),
                    // has been misleading, at least in the past (closed Issue #48492)
                    Applicability::MaybeIncorrect
                )
                .emit();
        }
        let in_span = self.prev_span;
        self.check_for_for_in_in_typo(in_span);
        let expr = self.parse_expr_res(Restrictions::NO_STRUCT_LITERAL, None)?;

        let pat = self.recover_parens_around_for_head(pat, &expr, begin_paren);

        let (iattrs, loop_block) = self.parse_inner_attrs_and_block()?;
        attrs.extend(iattrs);

        let hi = self.prev_span;
        Ok(self.mk_expr(span_lo.to(hi), ExprKind::ForLoop(pat, expr, loop_block, opt_label), attrs))
    }

    /// Parses a `while` or `while let` expression (`while` token already eaten).
    fn parse_while_expr(
        &mut self,
        opt_label: Option<Label>,
        span_lo: Span,
        mut attrs: ThinVec<Attribute>
    ) -> PResult<'a, P<Expr>> {
        let cond = self.parse_cond_expr()?;
        let (iattrs, body) = self.parse_inner_attrs_and_block()?;
        attrs.extend(iattrs);
        let span = span_lo.to(body.span);
        Ok(self.mk_expr(span, ExprKind::While(cond, body, opt_label), attrs))
    }

    /// Parse `loop {...}`, `loop` token already eaten.
    fn parse_loop_expr(
        &mut self,
        opt_label: Option<Label>,
        span_lo: Span,
        mut attrs: ThinVec<Attribute>
    ) -> PResult<'a, P<Expr>> {
        let (iattrs, body) = self.parse_inner_attrs_and_block()?;
        attrs.extend(iattrs);
        let span = span_lo.to(body.span);
        Ok(self.mk_expr(span, ExprKind::Loop(body, opt_label), attrs))
    }

    fn eat_label(&mut self) -> Option<Label> {
        if let Some(ident) = self.token.lifetime() {
            let span = self.token.span;
            self.bump();
            Some(Label { ident: Ident::new(ident.name, span) })
        } else {
            None
        }
    }

    // `match` token already eaten
    fn parse_match_expr(&mut self, mut attrs: ThinVec<Attribute>) -> PResult<'a, P<Expr>> {
        let match_span = self.prev_span;
        let lo = self.prev_span;
        let discriminant = self.parse_expr_res(Restrictions::NO_STRUCT_LITERAL, None)?;
        if let Err(mut e) = self.expect(&token::OpenDelim(token::Brace)) {
            if self.token == token::Semi {
                e.span_suggestion_short(
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
                    let span = lo.to(self.token.span);
                    if self.token == token::CloseDelim(token::Brace) {
                        self.bump();
                    }
                    return Ok(self.mk_expr(span, ExprKind::Match(discriminant, arms), attrs));
                }
            }
        }
        let hi = self.token.span;
        self.bump();
        return Ok(self.mk_expr(lo.to(hi), ExprKind::Match(discriminant, arms), attrs));
    }

    crate fn parse_arm(&mut self) -> PResult<'a, Arm> {
        let attrs = self.parse_outer_attributes()?;
        let lo = self.token.span;
        let pats = self.parse_pats()?;
        let guard = if self.eat_keyword(kw::If) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        let arrow_span = self.token.span;
        self.expect(&token::FatArrow)?;
        let arm_start_span = self.token.span;

        let expr = self.parse_expr_res(Restrictions::STMT_EXPR, None)
            .map_err(|mut err| {
                err.span_label(arrow_span, "while parsing the `match` arm starting here");
                err
            })?;

        let require_comma = classify::expr_requires_semi_to_be_stmt(&expr)
            && self.token != token::CloseDelim(token::Brace);

        let hi = self.token.span;

        if require_comma {
            let cm = self.sess.source_map();
            self.expect_one_of(&[token::Comma], &[token::CloseDelim(token::Brace)])
                .map_err(|mut err| {
                    match (cm.span_to_lines(expr.span), cm.span_to_lines(arm_start_span)) {
                        (Ok(ref expr_lines), Ok(ref arm_start_lines))
                        if arm_start_lines.lines[0].end_col == expr_lines.lines[0].end_col
                            && expr_lines.lines.len() == 2
                            && self.token == token::FatArrow => {
                            // We check whether there's any trailing code in the parse span,
                            // if there isn't, we very likely have the following:
                            //
                            // X |     &Y => "y"
                            //   |        --    - missing comma
                            //   |        |
                            //   |        arrow_span
                            // X |     &X => "x"
                            //   |      - ^^ self.token.span
                            //   |      |
                            //   |      parsed until here as `"y" & X`
                            err.span_suggestion_short(
                                cm.next_point(arm_start_span),
                                "missing a comma here to end this `match` arm",
                                ",".to_owned(),
                                Applicability::MachineApplicable
                            );
                        }
                        _ => {
                            err.span_label(arrow_span,
                                           "while parsing the `match` arm starting here");
                        }
                    }
                    err
                })?;
        } else {
            self.eat(&token::Comma);
        }
