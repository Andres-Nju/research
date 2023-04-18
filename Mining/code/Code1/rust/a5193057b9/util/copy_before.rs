pub fn copy(src: &Path, dst: &Path) {
    let res = fs::hard_link(src, dst);
    let res = res.or_else(|_| fs::copy(src, dst).map(|_| ()));
    if let Err(e) = res {
        panic!("failed to copy `{}` to `{}`: {}", src.display(),
               dst.display(), e)
    }
}
