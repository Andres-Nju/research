    fn parse_tpl_elements(
        &mut self,
        is_tagged: bool,
    ) -> PResult<'a, (Vec<Box<Expr>>, Vec<TplElement>)> {
        let mut exprs = vec![];

        let cur_elem = self.parse_tpl_element(is_tagged)?;
        let mut is_tail = cur_elem.tail;
        let mut quasis = vec![cur_elem];

        while !is_tail {
            expect!("${");
            exprs.push(self.include_in_expr(true).parse_expr()?);
            expect!('}');
            let elem = self.parse_tpl_element(is_tagged)?;
            is_tail = elem.tail;
            quasis.push(elem);
        }

        Ok((exprs, quasis))
    }

    fn parse_tagged_tpl(
        &mut self,
        tag: Box<Expr>,
        type_params: Option<TsTypeParamInstantiation>,
    ) -> PResult<'a, TaggedTpl> {
        let start = tag.span().lo();

        assert_and_bump!('`');

        let (exprs, quasis) = self.parse_tpl_elements(false)?;

        expect!('`');

        let span = span!(start);
        Ok(TaggedTpl {
            span,
            tag,
            exprs,
            type_params,
            quasis,
        })
    }
