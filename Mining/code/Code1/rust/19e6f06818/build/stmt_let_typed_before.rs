    fn stmt_let_typed(&self,
                      sp: Span,
                      mutbl: bool,
                      ident: ast::Ident,
                      typ: P<ast::Ty>,
                      ex: P<ast::Expr>)
                      -> P<ast::Stmt> {
        let pat = if mutbl {
            let binding_mode = ast::BindingMode::ByValue(ast::Mutability::Mutable);
            self.pat_ident_binding_mode(sp, ident, binding_mode)
        } else {
            self.pat_ident(sp, ident)
        };
        let local = P(ast::Local {
            pat: pat,
            ty: Some(typ),
            init: Some(ex),
            id: ast::DUMMY_NODE_ID,
            span: sp,
            attrs: ast::ThinVec::new(),
        });
        P(ast::Stmt {
            id: ast::DUMMY_NODE_ID,
            node: ast::StmtKind::Local(local),
            span: sp,
        })
    }
