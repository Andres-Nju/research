    fn check_inline(&self, attr: &ast::Attribute, target: Target) {
        if target != Target::Fn {
            struct_span_err!(self.sess, attr.span, E0518, "attribute should be applied to function")
                .span_label(attr.span, &format!("requires a function"))
                .emit();
        }
    }
