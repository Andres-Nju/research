            fn process_name(
                dict: &mut TaggedDictBuilder,
                cmd_name: &str,
                registry: CommandRegistry,
                rest: Vec<Tagged<String>>,
                name: Tag,
            ) -> Result<(), ShellError> {
                let document_tag = rest[0].tag.clone();
                let value = command_dict(
                    registry.get_command(&cmd_name).ok_or_else(|| {
                        ShellError::labeled_error(
                            format!("Could not load {}", cmd_name),
                            "could not load command",
                            document_tag,
                        )
                    })?,
                    name,
                );

                dict.insert_untagged("name", cmd_name);
                dict.insert_untagged(
                    "description",
                    get_data_by_key(&value, "usage".spanned_unknown())
                        .ok_or_else(|| {
                            ShellError::labeled_error(
                                "Expected a usage key",
                                "expected a 'usage' key",
                                &value.tag,
                            )
                        })?
                        .as_string()?,
                );

                Ok(())
            }

