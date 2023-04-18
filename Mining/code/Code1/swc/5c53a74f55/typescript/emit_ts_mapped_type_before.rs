    fn emit_ts_mapped_type(&mut self, n: &TsMappedType) -> Result {
        self.emit_leading_comments_of_span(n.span(), false)?;

        punct!("{");
        self.wr.write_line()?;
        self.wr.increase_indent()?;

        match n.readonly {
            None => {}
            Some(tpm) => match tpm {
                TruePlusMinus::True => {
                    keyword!("readonly");
                    space!();
                }
                TruePlusMinus::Plus => {
                    punct!("+");
                    keyword!("readonly");
                    space!();
                }
                TruePlusMinus::Minus => {
                    punct!("-");
                    keyword!("readonly");
                    space!();
                }
            },
        }

        punct!("[");
        emit!(n.type_param.name);

        if let Some(constraints) = &n.type_param.constraint {
            space!();
            keyword!("in");
            space!();
        }

        if let Some(default) = &n.type_param.default {
            formatting_space!();
            punct!("=");
            formatting_space!();
            emit!(default);
        }

        emit!(n.type_param.constraint);

        punct!("]");

        match n.optional {
            None => {}
            Some(tpm) => match tpm {
                TruePlusMinus::True => {
                    punct!("?");
                }
                TruePlusMinus::Plus => {
                    punct!("+");
                    punct!("/");
                }
                TruePlusMinus::Minus => {
                    punct!("-");
                    punct!("?");
                }
            },
        }

        punct!(":");
        space!();
        emit!(n.type_ann);
        formatting_semi!();

        self.wr.write_line()?;
        self.wr.decrease_indent()?;
        punct!("}");
    }
