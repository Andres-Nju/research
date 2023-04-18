    pub fn emit_method_prop(&mut self, node: &MethodProp) -> Result {
        self.emit_leading_comments_of_pos(node.span().lo())?;

        if node.function.is_generator {
            punct!("*");
        }

        emit!(node.key);
        formatting_space!();
        // TODO
        self.emit_fn_trailing(&node.function)?;
    }
