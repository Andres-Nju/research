    fn rm(
        &self,
        RemoveArgs {
            rest: targets,
            recursive,
            trash: _trash,
            permanent: _permanent,
            force: _force,
        }: RemoveArgs,
        name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let name_tag = name;

        if targets.is_empty() {
            return Err(ShellError::labeled_error(
                "rm requires target paths",
                "needs parameter",
                name_tag,
            ));
        }

        let path = Path::new(path);
        let mut all_targets: HashMap<PathBuf, Tag> = HashMap::new();
        for target in targets {
            let all_dots = target
                .item
                .to_str()
                .map_or(false, |v| v.chars().all(|c| c == '.'));

            if all_dots {
                return Err(ShellError::labeled_error(
                    "Cannot remove any parent directory",
                    "cannot remove any parent directory",
                    target.tag,
                ));
            }

            let path = path.join(&target.item);
            match glob::glob(&path.to_string_lossy()) {
                Ok(files) => {
                    for file in files {
                        match file {
                            Ok(ref f) => {
                                all_targets
                                    .entry(f.clone())
                                    .or_insert_with(|| target.tag.clone());
                            }
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    format!("Could not remove {:}", path.to_string_lossy()),
                                    e.to_string(),
                                    &target.tag,
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(ShellError::labeled_error(
                        e.to_string(),
                        e.to_string(),
                        &name_tag,
                    ))
                }
            };
        }

        if all_targets.is_empty() && !_force.item {
            return Err(ShellError::labeled_error(
                "No valid paths",
                "no valid paths",
                name_tag,
            ));
        }

        Ok(
            futures::stream::iter(all_targets.into_iter().map(move |(f, tag)| {
                let is_empty = || match f.read_dir() {
                    Ok(mut p) => p.next().is_none(),
                    Err(_) => false,
                };

                if let Ok(metadata) = f.symlink_metadata() {
                    #[cfg(unix)]
                    let is_socket = metadata.file_type().is_socket();
                    #[cfg(not(unix))]
                    let is_socket = false;

                    if metadata.is_file()
                        || metadata.file_type().is_symlink()
                        || recursive.item
                        || is_socket
                        || is_empty()
                    {
                        let result;
                        #[cfg(feature = "trash-support")]
                        {
                            let rm_always_trash = nu_data::config::config(Tag::unknown())?
                                .get("rm_always_trash")
                                .map(|val| val.is_true())
                                .unwrap_or(false);
                            result = if _trash.item || (rm_always_trash && !_permanent.item) {
                                trash::delete(&f).map_err(|e: trash::Error| {
                                    Error::new(ErrorKind::Other, format!("{:?}", e))
                                })
                            } else if metadata.is_file() {
                                std::fs::remove_file(&f)
                            } else {
                                std::fs::remove_dir_all(&f)
                            };
                        }
                        #[cfg(not(feature = "trash-support"))]
                        {
                            result = if metadata.is_file() || is_socket {
                                std::fs::remove_file(&f)
                            } else {
                                std::fs::remove_dir_all(&f)
                            };
                        }

                        if let Err(e) = result {
                            let msg =
                                format!("Could not delete because: {:}\nTry '--trash' flag", e);
                            Err(ShellError::labeled_error(msg, e.to_string(), tag))
                        } else {
                            let val = format!("deleted {:}", f.to_string_lossy()).into();
                            Ok(ReturnSuccess::Value(val))
                        }
                    } else {
                        let msg =
                            format!("Cannot remove {:}. try --recursive", f.to_string_lossy());
                        Err(ShellError::labeled_error(
                            msg,
                            "cannot remove non-empty directory",
                            tag,
                        ))
                    }
                } else {
                    let msg = format!("no such file or directory: {:}", f.to_string_lossy());
                    Err(ShellError::labeled_error(
                        msg,
                        "no such file or directory",
                        tag,
                    ))
                }
            }))
            .to_output_stream(),
        )
    }
