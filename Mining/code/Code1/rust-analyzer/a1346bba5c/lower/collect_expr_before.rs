    fn collect_expr(&mut self, expr: ast::Expr) -> ExprId {
        let syntax_ptr = AstPtr::new(&expr);
        match expr {
            ast::Expr::IfExpr(e) => {
                let then_branch = self.collect_block_opt(e.then_branch());

                let else_branch = e.else_branch().map(|b| match b {
                    ast::ElseBranch::Block(it) => self.collect_block(it),
                    ast::ElseBranch::IfExpr(elif) => {
                        let expr: ast::Expr = ast::Expr::cast(elif.syntax().clone()).unwrap();
                        self.collect_expr(expr)
                    }
                });

                let condition = match e.condition() {
                    None => self.missing_expr(),
                    Some(condition) => match condition.pat() {
                        None => self.collect_expr_opt(condition.expr()),
                        // if let -- desugar to match
                        Some(pat) => {
                            tested_by!(infer_resolve_while_let);
                            let pat = self.collect_pat(pat);
                            let match_expr = self.collect_expr_opt(condition.expr());
                            let placeholder_pat = self.missing_pat();
                            let arms = vec![
                                MatchArm { pats: vec![pat], expr: then_branch, guard: None },
                                MatchArm {
                                    pats: vec![placeholder_pat],
                                    expr: else_branch.unwrap_or_else(|| self.empty_block()),
                                    guard: None,
                                },
                            ];
                            return self
                                .alloc_expr(Expr::Match { expr: match_expr, arms }, syntax_ptr);
                        }
                    },
                };

                self.alloc_expr(Expr::If { condition, then_branch, else_branch }, syntax_ptr)
            }
            ast::Expr::TryBlockExpr(e) => {
                let body = self.collect_block_opt(e.body());
                self.alloc_expr(Expr::TryBlock { body }, syntax_ptr)
            }
            ast::Expr::BlockExpr(e) => self.collect_block(e),
            ast::Expr::LoopExpr(e) => {
                let body = self.collect_block_opt(e.loop_body());
                self.alloc_expr(Expr::Loop { body }, syntax_ptr)
            }
            ast::Expr::WhileExpr(e) => {
                let body = self.collect_block_opt(e.loop_body());

                let condition = match e.condition() {
                    None => self.missing_expr(),
                    Some(condition) => match condition.pat() {
                        None => self.collect_expr_opt(condition.expr()),
                        // if let -- desugar to match
                        Some(pat) => {
                            let pat = self.collect_pat(pat);
                            let match_expr = self.collect_expr_opt(condition.expr());
                            let placeholder_pat = self.missing_pat();
                            let break_ = self.alloc_expr_desugared(Expr::Break { expr: None });
                            let arms = vec![
                                MatchArm { pats: vec![pat], expr: body, guard: None },
                                MatchArm { pats: vec![placeholder_pat], expr: break_, guard: None },
                            ];
                            let match_expr =
                                self.alloc_expr_desugared(Expr::Match { expr: match_expr, arms });
                            return self.alloc_expr(Expr::Loop { body: match_expr }, syntax_ptr);
                        }
                    },
                };

                self.alloc_expr(Expr::While { condition, body }, syntax_ptr)
            }
            ast::Expr::ForExpr(e) => {
                let iterable = self.collect_expr_opt(e.iterable());
                let pat = self.collect_pat_opt(e.pat());
                let body = self.collect_block_opt(e.loop_body());
                self.alloc_expr(Expr::For { iterable, pat, body }, syntax_ptr)
            }
            ast::Expr::CallExpr(e) => {
                let callee = self.collect_expr_opt(e.expr());
                let args = if let Some(arg_list) = e.arg_list() {
                    arg_list.args().map(|e| self.collect_expr(e)).collect()
                } else {
                    Vec::new()
                };
                self.alloc_expr(Expr::Call { callee, args }, syntax_ptr)
            }
            ast::Expr::MethodCallExpr(e) => {
                let receiver = self.collect_expr_opt(e.expr());
                let args = if let Some(arg_list) = e.arg_list() {
                    arg_list.args().map(|e| self.collect_expr(e)).collect()
                } else {
                    Vec::new()
                };
                let method_name = e.name_ref().map(|nr| nr.as_name()).unwrap_or_else(Name::missing);
                let generic_args = e.type_arg_list().and_then(GenericArgs::from_ast);
                self.alloc_expr(
                    Expr::MethodCall { receiver, method_name, args, generic_args },
                    syntax_ptr,
                )
            }
            ast::Expr::MatchExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                let arms = if let Some(match_arm_list) = e.match_arm_list() {
                    match_arm_list
                        .arms()
                        .map(|arm| MatchArm {
                            pats: arm.pats().map(|p| self.collect_pat(p)).collect(),
                            expr: self.collect_expr_opt(arm.expr()),
                            guard: arm
                                .guard()
                                .and_then(|guard| guard.expr())
                                .map(|e| self.collect_expr(e)),
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                self.alloc_expr(Expr::Match { expr, arms }, syntax_ptr)
            }
            ast::Expr::PathExpr(e) => {
                let path = e
                    .path()
                    .and_then(|path| self.expander.parse_path(path))
                    .map(Expr::Path)
                    .unwrap_or(Expr::Missing);
                self.alloc_expr(path, syntax_ptr)
            }
            ast::Expr::ContinueExpr(_e) => {
                // FIXME: labels
                self.alloc_expr(Expr::Continue, syntax_ptr)
            }
            ast::Expr::BreakExpr(e) => {
                let expr = e.expr().map(|e| self.collect_expr(e));
                self.alloc_expr(Expr::Break { expr }, syntax_ptr)
            }
            ast::Expr::ParenExpr(e) => {
                let inner = self.collect_expr_opt(e.expr());
                // make the paren expr point to the inner expression as well
                let src = self.expander.to_source(Either::A(syntax_ptr));
                self.source_map.expr_map.insert(src, inner);
                inner
            }
            ast::Expr::ReturnExpr(e) => {
                let expr = e.expr().map(|e| self.collect_expr(e));
                self.alloc_expr(Expr::Return { expr }, syntax_ptr)
            }
            ast::Expr::RecordLit(e) => {
                let path = e.path().and_then(|path| self.expander.parse_path(path));
                let mut field_ptrs = Vec::new();
                let record_lit = if let Some(nfl) = e.record_field_list() {
                    let fields = nfl
                        .fields()
                        .inspect(|field| field_ptrs.push(AstPtr::new(field)))
                        .map(|field| RecordLitField {
                            name: field
                                .name_ref()
                                .map(|nr| nr.as_name())
                                .unwrap_or_else(Name::missing),
                            expr: if let Some(e) = field.expr() {
                                self.collect_expr(e)
                            } else if let Some(nr) = field.name_ref() {
                                // field shorthand
                                self.alloc_expr_field_shorthand(
                                    Expr::Path(Path::from_name_ref(&nr)),
                                    AstPtr::new(&field),
                                )
                            } else {
                                self.missing_expr()
                            },
                        })
                        .collect();
                    let spread = nfl.spread().map(|s| self.collect_expr(s));
                    Expr::RecordLit { path, fields, spread }
                } else {
                    Expr::RecordLit { path, fields: Vec::new(), spread: None }
                };

                let res = self.alloc_expr(record_lit, syntax_ptr);
                for (i, ptr) in field_ptrs.into_iter().enumerate() {
                    self.source_map.field_map.insert((res, i), ptr);
                }
                res
            }
            ast::Expr::FieldExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                let name = match e.field_access() {
                    Some(kind) => kind.as_name(),
                    _ => Name::missing(),
                };
                self.alloc_expr(Expr::Field { expr, name }, syntax_ptr)
            }
            ast::Expr::AwaitExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                self.alloc_expr(Expr::Await { expr }, syntax_ptr)
            }
            ast::Expr::TryExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                self.alloc_expr(Expr::Try { expr }, syntax_ptr)
            }
            ast::Expr::CastExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                let type_ref = TypeRef::from_ast_opt(e.type_ref());
                self.alloc_expr(Expr::Cast { expr, type_ref }, syntax_ptr)
            }
            ast::Expr::RefExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                let mutability = Mutability::from_mutable(e.is_mut());
                self.alloc_expr(Expr::Ref { expr, mutability }, syntax_ptr)
            }
            ast::Expr::PrefixExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                if let Some(op) = e.op_kind() {
                    self.alloc_expr(Expr::UnaryOp { expr, op }, syntax_ptr)
                } else {
                    self.alloc_expr(Expr::Missing, syntax_ptr)
                }
            }
            ast::Expr::LambdaExpr(e) => {
                let mut args = Vec::new();
                let mut arg_types = Vec::new();
                if let Some(pl) = e.param_list() {
                    for param in pl.params() {
                        let pat = self.collect_pat_opt(param.pat());
                        let type_ref = param.ascribed_type().map(TypeRef::from_ast);
                        args.push(pat);
                        arg_types.push(type_ref);
                    }
                }
                let body = self.collect_expr_opt(e.body());
                self.alloc_expr(Expr::Lambda { args, arg_types, body }, syntax_ptr)
            }
            ast::Expr::BinExpr(e) => {
                let lhs = self.collect_expr_opt(e.lhs());
                let rhs = self.collect_expr_opt(e.rhs());
                let op = e.op_kind().map(BinaryOp::from);
                self.alloc_expr(Expr::BinaryOp { lhs, rhs, op }, syntax_ptr)
            }
            ast::Expr::TupleExpr(e) => {
                let exprs = e.exprs().map(|expr| self.collect_expr(expr)).collect();
                self.alloc_expr(Expr::Tuple { exprs }, syntax_ptr)
            }
            ast::Expr::BoxExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                self.alloc_expr(Expr::Box { expr }, syntax_ptr)
            }

            ast::Expr::ArrayExpr(e) => {
                let kind = e.kind();

                match kind {
                    ArrayExprKind::ElementList(e) => {
                        let exprs = e.map(|expr| self.collect_expr(expr)).collect();
                        self.alloc_expr(Expr::Array(Array::ElementList(exprs)), syntax_ptr)
                    }
                    ArrayExprKind::Repeat { initializer, repeat } => {
                        let initializer = self.collect_expr_opt(initializer);
                        let repeat = self.collect_expr_opt(repeat);
                        self.alloc_expr(
                            Expr::Array(Array::Repeat { initializer, repeat }),
                            syntax_ptr,
                        )
                    }
                }
            }

            ast::Expr::Literal(e) => {
                let lit = match e.kind() {
                    LiteralKind::IntNumber { suffix } => {
                        let known_name = suffix.and_then(|it| BuiltinInt::from_suffix(&it));

                        Literal::Int(Default::default(), known_name)
                    }
                    LiteralKind::FloatNumber { suffix } => {
                        let known_name = suffix.and_then(|it| BuiltinFloat::from_suffix(&it));

                        Literal::Float(Default::default(), known_name)
                    }
                    LiteralKind::ByteString => Literal::ByteString(Default::default()),
                    LiteralKind::String => Literal::String(Default::default()),
                    LiteralKind::Byte => Literal::Int(Default::default(), Some(BuiltinInt::U8)),
                    LiteralKind::Bool => Literal::Bool(Default::default()),
                    LiteralKind::Char => Literal::Char(Default::default()),
                };
                self.alloc_expr(Expr::Literal(lit), syntax_ptr)
            }
            ast::Expr::IndexExpr(e) => {
                let base = self.collect_expr_opt(e.base());
                let index = self.collect_expr_opt(e.index());
                self.alloc_expr(Expr::Index { base, index }, syntax_ptr)
            }

            // FIXME implement HIR for these:
            ast::Expr::Label(_e) => self.alloc_expr(Expr::Missing, syntax_ptr),
            ast::Expr::RangeExpr(_e) => self.alloc_expr(Expr::Missing, syntax_ptr),
            ast::Expr::MacroCall(e) => match self.expander.enter_expand(self.db, e) {
                Some((mark, expansion)) => {
                    let id = self.collect_expr(expansion);
                    self.expander.exit(self.db, mark);
                    id
                }
                None => self.alloc_expr(Expr::Missing, syntax_ptr),
            },
        }
    }
