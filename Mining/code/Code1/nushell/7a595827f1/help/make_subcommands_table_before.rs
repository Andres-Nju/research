            fn make_subcommands_table(
                subcommand_names: &mut Vec<String>,
                cmd_name: &str,
                registry: CommandRegistry,
                rest: Vec<Tagged<String>>,
                name: Tag,
            ) -> Result<Value, ShellError> {
                let (matching, not_matching) = subcommand_names
                    .drain(..)
                    .partition(|subcommand_name| subcommand_name.starts_with(cmd_name));
                *subcommand_names = not_matching;
                Ok(if !matching.is_empty() {
                    UntaggedValue::table(
                        &(matching
                            .into_iter()
                            .map(|cmd_name: String| -> Result<_, ShellError> {
                                let mut short_desc = TaggedDictBuilder::new(name.clone());
                                process_name(
                                    &mut short_desc,
                                    &cmd_name,
                                    registry.clone(),
                                    rest.clone(),
                                    name.clone(),
                                )?;
                                Ok(short_desc.into_value())
                            })
                            .collect::<Result<Vec<_>, _>>()?[..]),
                    )
                    .into_value(name)
                } else {
                    UntaggedValue::nothing().into_value(name)
                })
            }
