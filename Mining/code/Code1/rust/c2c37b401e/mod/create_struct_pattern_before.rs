    fn create_struct_pattern
        (&self,
         cx: &mut ExtCtxt,
         struct_path: ast::Path,
         struct_def: &'a VariantData,
         prefix: &str,
         mutbl: ast::Mutability)
         -> (P<ast::Pat>, Vec<(Span, Option<Ident>, P<Expr>, &'a [ast::Attribute])>) {
        let mut paths = Vec::new();
        let mut ident_exprs = Vec::new();
        for (i, struct_field) in struct_def.fields().iter().enumerate() {
            let sp = Span { expn_id: self.span.expn_id, ..struct_field.span };
            let ident = cx.ident_of(&format!("{}_{}", prefix, i));
            paths.push(codemap::Spanned {
                span: sp,
                node: ident,
            });
            let val = cx.expr_deref(sp, cx.expr_path(cx.path_ident(sp, ident)));
            let val = cx.expr(sp, ast::ExprKind::Paren(val));
            ident_exprs.push((sp, struct_field.ident, val, &struct_field.attrs[..]));
        }

        let subpats = self.create_subpatterns(cx, paths, mutbl);
        let pattern = match *struct_def {
            VariantData::Struct(..) => {
                let field_pats = subpats.into_iter()
                    .zip(&ident_exprs)
                    .map(|(pat, &(sp, ident, ..))| {
                        if ident.is_none() {
                            cx.span_bug(sp, "a braced struct with unnamed fields in `derive`");
                        }
                        codemap::Spanned {
                            span: pat.span,
                            node: ast::FieldPat {
                                ident: ident.unwrap(),
                                pat: pat,
                                is_shorthand: false,
                            },
                        }
                    })
                    .collect();
                cx.pat_struct(self.span, struct_path, field_pats)
            }
            VariantData::Tuple(..) => {
                cx.pat_tuple_struct(self.span, struct_path, subpats)
            }
            VariantData::Unit(..) => {
                cx.pat_path(self.span, struct_path)
            }
        };

        (pattern, ident_exprs)
    }
