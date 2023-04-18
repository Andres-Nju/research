    fn check_inline(&self, attr: &ast::Attribute, target: Target) {
        if target != Target::Fn {
            span_err!(self.sess, attr.span, E0518, "attribute should be applied to function");
        }
    }
