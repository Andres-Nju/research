fn cleanup_accounts_path(account_path: &std::path::Path) {
    if let Err(e) = std::fs::remove_dir_all(account_path) {
        warn!(
            "encountered error removing accounts path: {:?}: {}",
            account_path, e
        );
    }
}
