    pub(super) fn parse_fn_param_pat(&mut self) -> PResult<'a, P<Pat>> {
        self.recover_leading_vert("not allowed in a parameter pattern");
        let pat = self.parse_pat_with_or(PARAM_EXPECTED, GateOr::No, RecoverComma::No)?;

        if let PatKind::Or(..) = &pat.node {
            self.ban_illegal_fn_param_or_pat(&pat);
        }

        Ok(pat)
    }

    /// Ban `A | B` immediately in a parameter pattern and suggest wrapping in parens.
    fn ban_illegal_fn_param_or_pat(&self, pat: &Pat) {
        let msg = "wrap the pattern in parenthesis";
        let fix = format!("({})", pprust::pat_to_string(pat));
        self.struct_span_err(pat.span, "an or-pattern parameter must be wrapped in parenthesis")
            .span_suggestion(pat.span, msg, fix, Applicability::MachineApplicable)
            .emit();
    }

    /// Parses a pattern, that may be a or-pattern (e.g. `Foo | Bar` in `Some(Foo | Bar)`).
    /// Corresponds to `pat<allow_top_alt>` in RFC 2535.
    fn parse_pat_with_or(
        &mut self,
        expected: Expected,
        gate_or: GateOr,
        rc: RecoverComma,
    ) -> PResult<'a, P<Pat>> {
        // Parse the first pattern.
        let first_pat = self.parse_pat(expected)?;
        self.maybe_recover_unexpected_comma(first_pat.span, rc)?;

        // If the next token is not a `|`,
        // this is not an or-pattern and we should exit here.
        if !self.check(&token::BinOp(token::Or)) && self.token != token::OrOr {
            return Ok(first_pat)
        }

        let lo = first_pat.span;
        let mut pats = vec![first_pat];
        while self.eat_or_separator() {
            let pat = self.parse_pat(expected).map_err(|mut err| {
                err.span_label(lo, "while parsing this or-pattern staring here");
                err
            })?;
            self.maybe_recover_unexpected_comma(pat.span, rc)?;
            pats.push(pat);
        }
        let or_pattern_span = lo.to(self.prev_span);

        // Feature gate the or-pattern if instructed:
        if gate_or == GateOr::Yes {
            self.sess.gated_spans.or_patterns.borrow_mut().push(or_pattern_span);
        }

        Ok(self.mk_pat(or_pattern_span, PatKind::Or(pats)))
    }
