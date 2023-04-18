fn op_remove(
  state: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  let inner = base.inner_as_remove().unwrap();
  let path = PathBuf::from(inner.path().unwrap());
  let recursive = inner.recursive();
  if !state.flags.allow_write {
    return odd_future(permission_denied());
  }
  blocking!(base.sync(), || {
    debug!("op_remove {}", path.display());
    let metadata = fs::metadata(&path)?;
    if metadata.is_file() {
      fs::remove_file(&path)?;
    } else {
      if recursive {
        remove_dir_all(&path)?;
      } else {
        fs::remove_dir(&path)?;
      }
    }
    Ok(empty_buf())
  })
}

// Prototype https://github.com/denoland/isolate/blob/golang/os.go#L171-L184
fn op_read_file(
  _config: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  let inner = base.inner_as_read_file().unwrap();
  let cmd_id = base.cmd_id();
  let filename = PathBuf::from(inner.filename().unwrap());
  debug!("op_read_file {}", filename.display());
  blocking!(base.sync(), || {
    let vec = fs::read(&filename)?;
    // Build the response message. memcpy data into inner.
    // TODO(ry) zero-copy.
    let builder = &mut FlatBufferBuilder::new();
    let data_off = builder.create_vector(vec.as_slice());
    let inner = msg::ReadFileRes::create(
      builder,
      &msg::ReadFileResArgs {
        data: Some(data_off),
        ..Default::default()
      },
    );
    Ok(serialize_response(
      cmd_id,
      builder,
      msg::BaseArgs {
        inner: Some(inner.as_union_value()),
        inner_type: msg::Any::ReadFileRes,
        ..Default::default()
      },
    ))
  })
}
