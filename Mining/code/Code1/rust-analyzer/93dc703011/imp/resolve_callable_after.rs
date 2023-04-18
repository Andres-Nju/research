    pub fn resolve_callable(
        &self,
        file_id: FileId,
        offset: TextUnit,
    ) -> Cancelable<Option<(FnDescriptor, Option<usize>)>> {
        let file = self.db.file_syntax(file_id);
        let syntax = file.syntax();

        // Find the calling expression and it's NameRef
        let calling_node = match FnCallNode::with_node(syntax, offset) {
            Some(node) => node,
            None => return Ok(None),
        };
        let name_ref = match calling_node.name_ref() {
            Some(name) => name,
            None => return Ok(None),
        };

        // Resolve the function's NameRef (NOTE: this isn't entirely accurate).
        let file_symbols = self.index_resolve(name_ref)?;
        for (fn_fiel_id, fs) in file_symbols {
            if fs.kind == FN_DEF {
                let fn_file = self.db.file_syntax(fn_fiel_id);
                if let Some(fn_def) = find_node_at_offset(fn_file.syntax(), fs.node_range.start()) {
                    if let Some(descriptor) = FnDescriptor::new(fn_def) {
                        // If we have a calling expression let's find which argument we are on
                        let mut current_parameter = None;

                        let num_params = descriptor.params.len();
                        let has_self = fn_def.param_list().and_then(|l| l.self_param()).is_some();

                        if num_params == 1 {
                            if !has_self {
                                current_parameter = Some(0);
                            }
                        } else if num_params > 1 {
                            // Count how many parameters into the call we are.
                            // TODO: This is best effort for now and should be fixed at some point.
                            // It may be better to see where we are in the arg_list and then check
                            // where offset is in that list (or beyond).
                            // Revisit this after we get documentation comments in.
                            if let Some(ref arg_list) = calling_node.arg_list() {
                                let start = arg_list.syntax().range().start();

                                let range_search = TextRange::from_to(start, offset);
                                let mut commas: usize = arg_list
                                    .syntax()
                                    .text()
                                    .slice(range_search)
                                    .to_string()
                                    .matches(',')
                                    .count();

                                // If we have a method call eat the first param since it's just self.
                                if has_self {
                                    commas += 1;
                                }

                                current_parameter = Some(commas);
                            }
                        }

                        return Ok(Some((descriptor, current_parameter)));
                    }
                }
            }
        }

        Ok(None)
    }
