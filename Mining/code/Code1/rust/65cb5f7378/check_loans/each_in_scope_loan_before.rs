    pub fn each_in_scope_loan<F>(&self, scope: region::CodeExtent, mut op: F) -> bool where
        F: FnMut(&Loan<'tcx>) -> bool,
    {
        //! Like `each_issued_loan()`, but only considers loans that are
        //! currently in scope.

        let tcx = self.tcx();
        self.each_issued_loan(scope.node_id(&tcx.region_maps), |loan| {
            if tcx.region_maps.is_subscope_of(scope, loan.kill_scope) {
                op(loan)
            } else {
                true
            }
        })
    }

    fn each_in_scope_loan_affecting_path<F>(&self,
                                            scope: region::CodeExtent,
                                            loan_path: &LoanPath<'tcx>,
                                            mut op: F)
                                            -> bool where
        F: FnMut(&Loan<'tcx>) -> bool,
    {
        //! Iterates through all of the in-scope loans affecting `loan_path`,
        //! calling `op`, and ceasing iteration if `false` is returned.

        // First, we check for a loan restricting the path P being used. This
        // accounts for borrows of P but also borrows of subpaths, like P.a.b.
        // Consider the following example:
        //
        //     let x = &mut a.b.c; // Restricts a, a.b, and a.b.c
        //     let y = a;          // Conflicts with restriction

        let loan_path = owned_ptr_base_path(loan_path);
        let cont = self.each_in_scope_loan(scope, |loan| {
            let mut ret = true;
            for restr_path in &loan.restricted_paths {
                if **restr_path == *loan_path {
                    if !op(loan) {
                        ret = false;
                        break;
                    }
                }
            }
            ret
        });

        if !cont {
            return false;
        }

        // Next, we must check for *loans* (not restrictions) on the path P or
        // any base path. This rejects examples like the following:
        //
        //     let x = &mut a.b;
        //     let y = a.b.c;
        //
        // Limiting this search to *loans* and not *restrictions* means that
        // examples like the following continue to work:
        //
        //     let x = &mut a.b;
        //     let y = a.c;

        let mut loan_path = loan_path;
        loop {
            match loan_path.kind {
                LpVar(_) | LpUpvar(_) => {
                    break;
                }
                LpDowncast(ref lp_base, _) |
                LpExtend(ref lp_base, _, _) => {
                    loan_path = &lp_base;
                }
            }

            let cont = self.each_in_scope_loan(scope, |loan| {
                if *loan.loan_path == *loan_path {
                    op(loan)
                } else {
                    true
                }
            });

            if !cont {
                return false;
            }
        }

        return true;
    }

    pub fn loans_generated_by(&self, node: ast::NodeId) -> Vec<usize> {
        //! Returns a vector of the loans that are generated as
        //! we enter `node`.

        let mut result = Vec::new();
        self.dfcx_loans.each_gen_bit(node, |loan_index| {
            result.push(loan_index);
            true
        });
        return result;
    }

    pub fn check_for_conflicting_loans(&self, node: ast::NodeId) {
        //! Checks to see whether any of the loans that are issued
        //! on entrance to `node` conflict with loans that have already been
        //! issued when we enter `node` (for example, we do not
        //! permit two `&mut` borrows of the same variable).
        //!
        //! (Note that some loans can be *issued* without necessarily
        //! taking effect yet.)

        debug!("check_for_conflicting_loans(node={:?})", node);

        let new_loan_indices = self.loans_generated_by(node);
        debug!("new_loan_indices = {:?}", new_loan_indices);

        for &new_loan_index in &new_loan_indices {
            self.each_issued_loan(node, |issued_loan| {
                let new_loan = &self.all_loans[new_loan_index];
                // Only report an error for the first issued loan that conflicts
                // to avoid O(n^2) errors.
                self.report_error_if_loans_conflict(issued_loan, new_loan)
            });
        }

        for (i, &x) in new_loan_indices.iter().enumerate() {
            let old_loan = &self.all_loans[x];
            for &y in &new_loan_indices[(i+1) ..] {
                let new_loan = &self.all_loans[y];
                self.report_error_if_loans_conflict(old_loan, new_loan);
            }
        }
    }

    pub fn report_error_if_loans_conflict(&self,
                                          old_loan: &Loan<'tcx>,
                                          new_loan: &Loan<'tcx>)
                                          -> bool {
        //! Checks whether `old_loan` and `new_loan` can safely be issued
        //! simultaneously.

        debug!("report_error_if_loans_conflict(old_loan={:?}, new_loan={:?})",
               old_loan,
               new_loan);

        // Should only be called for loans that are in scope at the same time.
        assert!(self.tcx().region_maps.scopes_intersect(old_loan.kill_scope,
                                                        new_loan.kill_scope));

        self.report_error_if_loan_conflicts_with_restriction(
            old_loan, new_loan, old_loan, new_loan) &&
        self.report_error_if_loan_conflicts_with_restriction(
            new_loan, old_loan, old_loan, new_loan)
    }

    pub fn report_error_if_loan_conflicts_with_restriction(&self,
                                                           loan1: &Loan<'tcx>,
                                                           loan2: &Loan<'tcx>,
                                                           old_loan: &Loan<'tcx>,
                                                           new_loan: &Loan<'tcx>)
                                                           -> bool {
        //! Checks whether the restrictions introduced by `loan1` would
        //! prohibit `loan2`. Returns false if an error is reported.

        debug!("report_error_if_loan_conflicts_with_restriction(\
                loan1={:?}, loan2={:?})",
               loan1,
               loan2);

        if compatible_borrow_kinds(loan1.kind, loan2.kind) {
            return true;
        }

        let loan2_base_path = owned_ptr_base_path_rc(&loan2.loan_path);
        for restr_path in &loan1.restricted_paths {
            if *restr_path != loan2_base_path { continue; }

            // If new_loan is something like `x.a`, and old_loan is something like `x.b`, we would
            // normally generate a rather confusing message (in this case, for multiple mutable
            // borrows):
            //
            //     error: cannot borrow `x.b` as mutable more than once at a time
            //     note: previous borrow of `x.a` occurs here; the mutable borrow prevents
            //     subsequent moves, borrows, or modification of `x.a` until the borrow ends
            //
            // What we want to do instead is get the 'common ancestor' of the two borrow paths and
            // use that for most of the message instead, giving is something like this:
            //
            //     error: cannot borrow `x` as mutable more than once at a time
            //     note: previous borrow of `x` occurs here (through borrowing `x.a`); the mutable
            //     borrow prevents subsequent moves, borrows, or modification of `x` until the
            //     borrow ends

            let common = new_loan.loan_path.common(&old_loan.loan_path);
            let (nl, ol, new_loan_msg, old_loan_msg) = {
                if new_loan.loan_path.has_fork(&old_loan.loan_path) && common.is_some() {
                    let nl = self.bccx.loan_path_to_string(&common.unwrap());
                    let ol = nl.clone();
                    let new_loan_msg = format!(" (via `{}`)",
                                               self.bccx.loan_path_to_string(
                                                   &new_loan.loan_path));
                    let old_loan_msg = format!(" (via `{}`)",
                                               self.bccx.loan_path_to_string(
                                                   &old_loan.loan_path));
                    (nl, ol, new_loan_msg, old_loan_msg)
                } else {
                    (self.bccx.loan_path_to_string(&new_loan.loan_path),
                     self.bccx.loan_path_to_string(&old_loan.loan_path),
                     String::new(),
                     String::new())
                }
            };

            let ol_pronoun = if new_loan.loan_path == old_loan.loan_path {
                "it".to_string()
            } else {
                format!("`{}`", ol)
            };

            // We want to assemble all the relevant locations for the error.
            //
            // 1. Where did the new loan occur.
            //    - if due to closure creation, where was the variable used in closure?
            // 2. Where did old loan occur.
            // 3. Where does old loan expire.

            let previous_end_span =
                self.tcx().map.span(old_loan.kill_scope.node_id(&self.tcx().region_maps))
                              .end_point();

            let mut err = match (new_loan.kind, old_loan.kind) {
                (ty::MutBorrow, ty::MutBorrow) => {
                    let mut err =struct_span_err!(self.bccx, new_loan.span, E0499,
                                                  "cannot borrow `{}`{} as mutable \
                                                  more than once at a time",
                                                  nl, new_loan_msg);
                    err.span_label(
                            old_loan.span,
                            &format!("first mutable borrow occurs here{}", old_loan_msg));
                    err.span_label(
                            new_loan.span,
                            &format!("second mutable borrow occurs here{}", new_loan_msg));
                    err.span_label(
                            previous_end_span,
                            &format!("first borrow ends here"));
                    err
                }

                (ty::UniqueImmBorrow, ty::UniqueImmBorrow) => {
                    let mut err = struct_span_err!(self.bccx, new_loan.span, E0524,
                                     "two closures require unique access to `{}` \
                                      at the same time",
                                     nl);
                    err.span_label(
                            old_loan.span,
                            &format!("first closure is constructed here"));
                    err.span_label(
                            new_loan.span,
                            &format!("second closure is constructed here"));
                    err.span_label(
                            previous_end_span,
                            &format!("borrow from first closure ends here"));
                    err
                }

                (ty::UniqueImmBorrow, _) => {
                    let mut err = struct_span_err!(self.bccx, new_loan.span, E0500,
                                                   "closure requires unique access to `{}` \
                                                   but {} is already borrowed{}",
                                                   nl, ol_pronoun, old_loan_msg);
                    err.span_label(
                            new_loan.span,
                            &format!("closure construction occurs here{}", new_loan_msg));
                    err.span_label(
                            old_loan.span,
                            &format!("borrow occurs here{}", old_loan_msg));
                    err.span_label(
                            previous_end_span,
                            &format!("borrow ends here"));
                    err
                }

                (_, ty::UniqueImmBorrow) => {
                    let mut err = struct_span_err!(self.bccx, new_loan.span, E0501,
                                                   "cannot borrow `{}`{} as {} because \
                                                   previous closure requires unique access",
                                                   nl, new_loan_msg, new_loan.kind.to_user_str());
                    err.span_label(
                            new_loan.span,
                            &format!("borrow occurs here{}", new_loan_msg));
                    err.span_label(
                            old_loan.span,
                            &format!("closure construction occurs here{}", old_loan_msg));
                    err.span_label(
                            previous_end_span,
                            &format!("borrow from closure ends here"));
                    err
                }

                (_, _) => {
                    let mut err = struct_span_err!(self.bccx, new_loan.span, E0502,
                                                   "cannot borrow `{}`{} as {} because \
                                                   {} is also borrowed as {}{}",
                                                   nl,
                                                   new_loan_msg,
                                                   new_loan.kind.to_user_str(),
                                                   ol_pronoun,
                                                   old_loan.kind.to_user_str(),
                                                   old_loan_msg);
                    err.span_label(
                            new_loan.span,
                            &format!("{} borrow occurs here{}",
                                     new_loan.kind.to_user_str(),
                                     new_loan_msg));
                    err.span_label(
                            old_loan.span,
                            &format!("{} borrow occurs here{}",
                                     old_loan.kind.to_user_str(),
                                     old_loan_msg));
                    err.span_label(
                            previous_end_span,
                            &format!("{} borrow ends here",
                                     old_loan.kind.to_user_str()));
                    err
                }
            };

            match new_loan.cause {
                euv::ClosureCapture(span) => {
                    err.span_label(
                        span,
                        &format!("borrow occurs due to use of `{}` in closure", nl));
                }
                _ => { }
            }

            match old_loan.cause {
                euv::ClosureCapture(span) => {
                    err.span_label(
                        span,
                        &format!("previous borrow occurs due to use of `{}` in closure",
                                 ol));
                }
                _ => { }
            }

            err.emit();
            return false;
        }

        true
    }

    fn consume_common(&self,
                      id: ast::NodeId,
                      span: Span,
                      cmt: mc::cmt<'tcx>,
                      mode: euv::ConsumeMode) {
        match opt_loan_path(&cmt) {
            Some(lp) => {
                let moved_value_use_kind = match mode {
                    euv::Copy => {
                        self.check_for_copy_of_frozen_path(id, span, &lp);
                        MovedInUse
                    }
                    euv::Move(_) => {
                        match self.move_data.kind_of_move_of_path(id, &lp) {
                            None => {
                                // Sometimes moves don't have a move kind;
                                // this either means that the original move
                                // was from something illegal to move,
                                // or was moved from referent of an unsafe
                                // pointer or something like that.
                                MovedInUse
                            }
                            Some(move_kind) => {
                                self.check_for_move_of_borrowed_path(id, span,
                                                                     &lp, move_kind);
                                if move_kind == move_data::Captured {
                                    MovedInCapture
                                } else {
                                    MovedInUse
                                }
                            }
                        }
                    }
                };

                self.check_if_path_is_moved(id, span, moved_value_use_kind, &lp);
            }
            None => { }
        }
    }
