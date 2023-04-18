pub fn copy(src: &Path, dst: &Path) {
    // A call to `hard_link` will fail if `dst` exists, so remove it if it
    // already exists so we can try to help `hard_link` succeed.
    let _ = fs::remove_file(&dst);

    // Attempt to "easy copy" by creating a hard link (symlinks don't work on
    // windows), but if that fails just fall back to a slow `copy` operation.
    let res = fs::hard_link(src, dst);
    let res = res.or_else(|_| fs::copy(src, dst).map(|_| ()));
    if let Err(e) = res {
        panic!("failed to copy `{}` to `{}`: {}", src.display(),
               dst.display(), e)
    }
}
