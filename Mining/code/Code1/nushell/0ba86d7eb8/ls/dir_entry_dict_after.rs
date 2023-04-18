pub(crate) fn dir_entry_dict(
    filename: &std::path::Path, // absolute path
    display_name: &str,         // gile name to be displayed
    metadata: Option<&std::fs::Metadata>,
    span: Span,
    long: bool,
    du: bool,
    ctrl_c: Option<Arc<AtomicBool>>,
) -> Result<Value, ShellError> {
    let mut cols = vec![];
    let mut vals = vec![];
    let mut file_type = "unknown";

    cols.push("name".into());
    vals.push(Value::String {
        val: display_name.to_string(),
        span,
    });

    if let Some(md) = metadata {
        file_type = get_file_type(md);
        cols.push("type".into());
        vals.push(Value::String {
            val: file_type.to_string(),
            span,
        });
    } else {
        cols.push("type".into());
        vals.push(Value::nothing(span));
    }

    if long {
        cols.push("target".into());
        if let Some(md) = metadata {
            if md.file_type().is_symlink() {
                if let Ok(path_to_link) = filename.read_link() {
                    vals.push(Value::String {
                        val: path_to_link.to_string_lossy().to_string(),
                        span,
                    });
                } else {
                    vals.push(Value::String {
                        val: "Could not obtain target file's path".to_string(),
                        span,
                    });
                }
            } else {
                vals.push(Value::nothing(span));
            }
        }
    }

    if long {
        if let Some(md) = metadata {
            cols.push("readonly".into());
            vals.push(Value::Bool {
                val: md.permissions().readonly(),
                span,
            });

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let mode = md.permissions().mode();
                cols.push("mode".into());
                vals.push(Value::String {
                    val: umask::Mode::from(mode).to_string(),
                    span,
                });

                let nlinks = md.nlink();
                cols.push("num_links".into());
                vals.push(Value::Int {
                    val: nlinks as i64,
                    span,
                });

                let inode = md.ino();
                cols.push("inode".into());
                vals.push(Value::Int {
                    val: inode as i64,
                    span,
                });

                cols.push("uid".into());
                if let Some(user) = users::get_user_by_uid(md.uid()) {
                    vals.push(Value::String {
                        val: user.name().to_string_lossy().into(),
                        span,
                    });
                } else {
                    vals.push(Value::nothing(span))
                }

                cols.push("group".into());
                if let Some(group) = users::get_group_by_gid(md.gid()) {
                    vals.push(Value::String {
                        val: group.name().to_string_lossy().into(),
                        span,
                    });
                } else {
                    vals.push(Value::nothing(span))
                }
            }
        }
    }

    cols.push("size".to_string());
    if let Some(md) = metadata {
        let zero_sized = file_type == "pipe"
            || file_type == "socket"
            || file_type == "char device"
            || file_type == "block device";

        if md.is_dir() {
            if du {
                let params = DirBuilder::new(Span { start: 0, end: 2 }, None, false, None, false);
                let dir_size = DirInfo::new(filename, &params, None, ctrl_c).get_size();

                vals.push(Value::Filesize {
                    val: dir_size as i64,
                    span,
                });
            } else {
                let dir_size: u64 = md.len();

                vals.push(Value::Filesize {
                    val: dir_size as i64,
                    span,
                });
            };
        } else if md.is_file() {
            vals.push(Value::Filesize {
                val: md.len() as i64,
                span,
            });
        } else if md.file_type().is_symlink() {
            if let Ok(symlink_md) = filename.symlink_metadata() {
                vals.push(Value::Filesize {
                    val: symlink_md.len() as i64,
                    span,
                });
            } else {
                vals.push(Value::nothing(span));
            }
        } else {
            let value = if zero_sized {
                Value::Filesize { val: 0, span }
            } else {
                Value::nothing(span)
            };
            vals.push(value);
        }
    } else {
        vals.push(Value::nothing(span));
    }

    if let Some(md) = metadata {
        if long {
            cols.push("created".to_string());
            {
                let mut val = Value::nothing(span);
                if let Ok(c) = md.created() {
                    if let Some(local) = try_convert_to_local_date_time(c) {
                        val = Value::Date {
                            val: local.with_timezone(local.offset()),
                            span,
                        };
                    }
                }
                vals.push(val);
            }

            cols.push("accessed".to_string());
            {
                let mut val = Value::nothing(span);
                if let Ok(a) = md.accessed() {
                    if let Some(local) = try_convert_to_local_date_time(a) {
                        val = Value::Date {
                            val: local.with_timezone(local.offset()),
                            span,
                        };
                    }
                }
                vals.push(val);
            }
        }

        cols.push("modified".to_string());
        {
            let mut val = Value::nothing(span);
            if let Ok(m) = md.modified() {
                if let Some(local) = try_convert_to_local_date_time(m) {
                    val = Value::Date {
                        val: local.with_timezone(local.offset()),
                        span,
                    };
                }
            }
            vals.push(val);
        }
    } else {
        if long {
            cols.push("created".to_string());
            vals.push(Value::nothing(span));

            cols.push("accessed".to_string());
            vals.push(Value::nothing(span));
        }

        cols.push("modified".to_string());
        vals.push(Value::nothing(span));
    }

    Ok(Value::Record { cols, vals, span })
}
