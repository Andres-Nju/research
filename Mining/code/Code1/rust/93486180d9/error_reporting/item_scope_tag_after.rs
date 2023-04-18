        fn item_scope_tag(item: &hir::Item) -> &'static str {
            match item.node {
                hir::ItemImpl(..) => "impl",
                hir::ItemStruct(..) => "struct",
                hir::ItemEnum(..) => "enum",
                hir::ItemTrait(..) => "trait",
                hir::ItemFn(..) => "function body",
                _ => "item"
            }
        }

        fn explain_span(tcx: &TyCtxt, heading: &str, span: Span)
                        -> (String, Option<Span>) {
            let lo = tcx.sess.codemap().lookup_char_pos_adj(span.lo);
            (format!("the {} at {}:{}", heading, lo.line, lo.col.to_usize()),
             Some(span))
        }

        let (description, span) = match region {
            ty::ReScope(scope) => {
                let new_string;
                let unknown_scope = || {
                    format!("{}unknown scope: {:?}{}.  Please report a bug.",
                            prefix, scope, suffix)
                };
                let span = match scope.span(&self.region_maps, &self.map) {
                    Some(s) => s,
                    None => {
                        err.note(&unknown_scope());
                        return;
                    }
                };
                let tag = match self.map.find(scope.node_id(&self.region_maps)) {
                    Some(ast_map::NodeBlock(_)) => "block",
                    Some(ast_map::NodeExpr(expr)) => match expr.node {
                        hir::ExprCall(..) => "call",
                        hir::ExprMethodCall(..) => "method call",
                        hir::ExprMatch(_, _, hir::MatchSource::IfLetDesugar { .. }) => "if let",
                        hir::ExprMatch(_, _, hir::MatchSource::WhileLetDesugar) =>  "while let",
                        hir::ExprMatch(_, _, hir::MatchSource::ForLoopDesugar) =>  "for",
                        hir::ExprMatch(..) => "match",
                        _ => "expression",
                    },
                    Some(ast_map::NodeStmt(_)) => "statement",
                    Some(ast_map::NodeItem(it)) => item_scope_tag(&it),
                    Some(_) | None => {
                        err.span_note(span, &unknown_scope());
                        return;
                    }
                };
                let scope_decorated_tag = match self.region_maps.code_extent_data(scope) {
                    region::CodeExtentData::Misc(_) => tag,
                    region::CodeExtentData::CallSiteScope { .. } => {
                        "scope of call-site for function"
                    }
                    region::CodeExtentData::ParameterScope { .. } => {
                        "scope of function body"
                    }
                    region::CodeExtentData::DestructionScope(_) => {
                        new_string = format!("destruction scope surrounding {}", tag);
                        &new_string[..]
                    }
                    region::CodeExtentData::Remainder(r) => {
                        new_string = format!("block suffix following statement {}",
                                             r.first_statement_index);
                        &new_string[..]
                    }
                };
                explain_span(self, scope_decorated_tag, span)
            }

            ty::ReFree(ref fr) => {
                let prefix = match fr.bound_region {
                    ty::BrAnon(idx) => {
                        format!("the anonymous lifetime #{} defined on", idx + 1)
                    }
                    ty::BrFresh(_) => "an anonymous lifetime defined on".to_owned(),
                    _ => {
                        format!("the lifetime {} as defined on",
                                fr.bound_region)
                    }
                };

                match self.map.find(fr.scope.node_id(&self.region_maps)) {
                    Some(ast_map::NodeBlock(ref blk)) => {
                        let (msg, opt_span) = explain_span(self, "block", blk.span);
                        (format!("{} {}", prefix, msg), opt_span)
                    }
                    Some(ast_map::NodeItem(it)) => {
                        let tag = item_scope_tag(&it);
                        let (msg, opt_span) = explain_span(self, tag, it.span);
                        (format!("{} {}", prefix, msg), opt_span)
                    }
                    Some(_) | None => {
                        // this really should not happen, but it does:
                        // FIXME(#27942)
                        (format!("{} unknown free region bounded by scope {:?}",
                                 prefix, fr.scope), None)
                    }
                }
            }

            ty::ReStatic => ("the static lifetime".to_owned(), None),

            ty::ReEmpty => ("the empty lifetime".to_owned(), None),

            ty::ReEarlyBound(ref data) => (data.name.to_string(), None),

            // FIXME(#13998) ReSkolemized should probably print like
            // ReFree rather than dumping Debug output on the user.
            //
            // We shouldn't really be having unification failures with ReVar
            // and ReLateBound though.
            ty::ReSkolemized(..) | ty::ReVar(_) | ty::ReLateBound(..) => {
                (format!("lifetime {:?}", region), None)
            }
