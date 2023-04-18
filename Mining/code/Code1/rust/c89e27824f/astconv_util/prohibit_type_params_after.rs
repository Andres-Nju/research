    pub fn prohibit_type_params(self, segments: &[ast::PathSegment]) {
        for segment in segments {
            for typ in segment.parameters.types() {
                struct_span_err!(self.sess, typ.span, E0109,
                                 "type parameters are not allowed on this type")
                    .span_label(typ.span, &format!("type parameter not allowed"))
                    .emit();
                break;
            }
            for lifetime in segment.parameters.lifetimes() {
                span_err!(self.sess, lifetime.span, E0110,
                          "lifetime parameters are not allowed on this type");
                break;
            }
            for binding in segment.parameters.bindings() {
                self.prohibit_projection(binding.span);
                break;
            }
        }
    }
