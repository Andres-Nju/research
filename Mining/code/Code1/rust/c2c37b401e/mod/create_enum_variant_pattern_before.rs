    fn create_enum_variant_pattern
        (&self,
         cx: &mut ExtCtxt,
         enum_ident: ast::Ident,
         variant: &'a ast::Variant,
         prefix: &str,
         mutbl: ast::Mutability)
         -> (P<ast::Pat>, Vec<(Span, Option<Ident>, P<Expr>, &'a [ast::Attribute])>) {
        let variant_ident = variant.node.name;
        let variant_path = cx.path(variant.span, vec![enum_ident, variant_ident]);
        self.create_struct_pattern(cx, variant_path, &variant.node.data, prefix, mutbl)
    }
