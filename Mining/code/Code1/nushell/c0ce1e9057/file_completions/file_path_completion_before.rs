pub fn file_path_completion(
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
) -> Vec<(nu_protocol::Span, String)> {
    let (base_dir_name, partial) = partial_from(partial);

    let base_dir = nu_path::expand_path_with(&base_dir_name, cwd);
    // This check is here as base_dir.read_dir() with base_dir == "" will open the current dir
    // which we don't want in this case (if we did, base_dir would already be ".")
    if base_dir == Path::new("") {
        return Vec::new();
    }

    if let Ok(result) = base_dir.read_dir() {
        return result
            .filter_map(|entry| {
                entry.ok().and_then(|entry| {
                    let mut file_name = entry.file_name().to_string_lossy().into_owned();
                    if matches(&partial, &file_name) {
                        let mut path = format!("{}{}", base_dir_name, file_name);
                        if entry.path().is_dir() {
                            path.push(SEP);
                            file_name.push(SEP);
                        }

                        if path.contains(' ') {
                            path = format!("\'{}\'", path);
                        }

                        Some((span, path))
                    } else {
                        None
                    }
                })
            })
            .collect();
    }

    Vec::new()
}
