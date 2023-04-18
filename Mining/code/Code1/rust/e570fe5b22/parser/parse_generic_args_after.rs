    fn parse_generic_args(&mut self) -> PResult<'a, (Vec<GenericArg>, Vec<TypeBinding>)> {
        let mut args = Vec::new();
        let mut bindings = Vec::new();
        let mut misplaced_assoc_ty_bindings: Vec<Span> = Vec::new();
        let mut assoc_ty_bindings: Vec<Span> = Vec::new();

        let args_lo = self.span;

        loop {
            if self.check_lifetime() && self.look_ahead(1, |t| !t.is_like_plus()) {
                // Parse lifetime argument.
                args.push(GenericArg::Lifetime(self.expect_lifetime()));
                misplaced_assoc_ty_bindings.append(&mut assoc_ty_bindings);
            } else if self.check_ident() && self.look_ahead(1, |t| t == &token::Eq) {
                // Parse associated type binding.
                let lo = self.span;
                let ident = self.parse_ident()?;
                self.bump();
                let ty = self.parse_ty()?;
                let span = lo.to(self.prev_span);
                bindings.push(TypeBinding {
                    id: ast::DUMMY_NODE_ID,
                    ident,
                    ty,
                    span,
                });
                assoc_ty_bindings.push(span);
            } else if self.check_const_arg() {
                // FIXME(const_generics): to distinguish between idents for types and consts,
                // we should introduce a GenericArg::Ident in the AST and distinguish when
                // lowering to the HIR. For now, idents for const args are not permitted.

                // Parse const argument.
                let expr = if let token::OpenDelim(token::Brace) = self.token {
                    self.parse_block_expr(None, self.span, BlockCheckMode::Default, ThinVec::new())?
                } else if self.token.is_ident() {
                    // FIXME(const_generics): to distinguish between idents for types and consts,
                    // we should introduce a GenericArg::Ident in the AST and distinguish when
                    // lowering to the HIR. For now, idents for const args are not permitted.
                    return Err(
                        self.fatal("identifiers may currently not be used for const generics")
                    );
                } else {
                    self.parse_literal_maybe_minus()?
                };
                let value = AnonConst {
                    id: ast::DUMMY_NODE_ID,
                    value: expr,
                };
                args.push(GenericArg::Const(value));
                misplaced_assoc_ty_bindings.append(&mut assoc_ty_bindings);
            } else if self.check_type() {
                // Parse type argument.
                args.push(GenericArg::Type(self.parse_ty()?));
                misplaced_assoc_ty_bindings.append(&mut assoc_ty_bindings);
            } else {
                break
            }

            if !self.eat(&token::Comma) {
                break
            }
        }

        // FIXME: we would like to report this in ast_validation instead, but we currently do not
        // preserve ordering of generic parameters with respect to associated type binding, so we
        // lose that information after parsing.
        if misplaced_assoc_ty_bindings.len() > 0 {
            let mut err = self.struct_span_err(
                args_lo.to(self.prev_span),
                "associated type bindings must be declared after generic parameters",
            );
            for span in misplaced_assoc_ty_bindings {
                err.span_label(
                    span,
                    "this associated type binding should be moved after the generic parameters",
                );
            }
            err.emit();
        }

        Ok((args, bindings))
    }

    /// Parses an optional where-clause and places it in `generics`.
    ///
    /// ```ignore (only-for-syntax-highlight)
    /// where T : Trait<U, V> + 'b, 'a : 'b
    /// ```
    fn parse_where_clause(&mut self) -> PResult<'a, WhereClause> {
        maybe_whole!(self, NtWhereClause, |x| x);

        let mut where_clause = WhereClause {
            id: ast::DUMMY_NODE_ID,
            predicates: Vec::new(),
            span: syntax_pos::DUMMY_SP,
        };

        if !self.eat_keyword(keywords::Where) {
            return Ok(where_clause);
        }
        let lo = self.prev_span;

        // We are considering adding generics to the `where` keyword as an alternative higher-rank
        // parameter syntax (as in `where<'a>` or `where<T>`. To avoid that being a breaking
        // change we parse those generics now, but report an error.
        if self.choose_generics_over_qpath() {
            let generics = self.parse_generics()?;
            self.struct_span_err(
                generics.span,
                "generic parameters on `where` clauses are reserved for future use",
            )
                .span_label(generics.span, "currently unsupported")
                .emit();
        }

        loop {
