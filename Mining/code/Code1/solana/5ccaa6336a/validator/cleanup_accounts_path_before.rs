fn cleanup_accounts_path(account_path: &std::path::Path) {
    if std::fs::remove_dir_all(account_path).is_err() {
        warn!(
            "encountered error removing accounts path: {:?}",
            account_path
        );
    }
}
