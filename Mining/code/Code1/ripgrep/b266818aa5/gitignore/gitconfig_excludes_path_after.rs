fn gitconfig_excludes_path() -> Option<PathBuf> {
    // git supports $HOME/.gitconfig and $XDG_CONFIG_HOME/git/config. Notably,
    // both can be active at the same time, where $HOME/.gitconfig takes
    // precedent. So if $HOME/.gitconfig defines a `core.excludesFile`, then
    // we're done.
    match gitconfig_home_contents().and_then(|x| parse_excludes_file(&x)) {
        Some(path) => return Some(path),
        None => {}
    }
    match gitconfig_xdg_contents().and_then(|x| parse_excludes_file(&x)) {
        Some(path) => return Some(path),
        None => {}
    }
    excludes_file_default()
}
