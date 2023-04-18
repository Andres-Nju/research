    fn emit_jsx_attr_or_spread(&mut self, node: &JSXAttrOrSpread) -> Result {
        match *node {
            JSXAttrOrSpread::JSXAttr(ref n) => emit!(n),
            JSXAttrOrSpread::SpreadElement(ref n) => emit!(n),
        }
    }
