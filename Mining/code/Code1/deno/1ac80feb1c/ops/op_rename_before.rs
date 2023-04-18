fn op_rename(
  state: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  if !state.flags.allow_write {
    return odd_future(permission_denied());
  }
  let inner = base.inner_as_rename().unwrap();
  let oldpath = PathBuf::from(inner.oldpath().unwrap());
  let newpath = PathBuf::from(inner.newpath().unwrap());
  blocking!(base.sync(), || -> OpResult {
    debug!("op_rename {} {}", oldpath.display(), newpath.display());
    fs::rename(&oldpath, &newpath)?;
    Ok(empty_buf())
  })
}

fn op_symlink(
  state: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  if !state.flags.allow_write {
    return odd_future(permission_denied());
  }
  // TODO Use type for Windows.
  if cfg!(windows) {
    panic!("symlink for windows is not yet implemented")
  }

  let inner = base.inner_as_symlink().unwrap();
  let oldname = PathBuf::from(inner.oldname().unwrap());
  let newname = PathBuf::from(inner.newname().unwrap());
  blocking!(base.sync(), || -> OpResult {
    debug!("op_symlink {} {}", oldname.display(), newname.display());
    #[cfg(any(unix))]
    std::os::unix::fs::symlink(&oldname, &newname)?;
    Ok(empty_buf())
  })
}
