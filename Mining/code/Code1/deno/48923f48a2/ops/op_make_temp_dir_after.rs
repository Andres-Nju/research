fn op_make_temp_dir(
  state: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  let base = Box::new(*base);
  let inner = base.inner_as_make_temp_dir().unwrap();
  let cmd_id = base.cmd_id();

  if !state.flags.allow_write {
    return odd_future(permission_denied());
  }

  let dir = inner.dir().map(PathBuf::from);
  let prefix = inner.prefix().map(String::from);
  let suffix = inner.suffix().map(String::from);

  blocking!(base.sync(), || -> OpResult {
    // TODO(piscisaureus): use byte vector for paths, not a string.
    // See https://github.com/denoland/deno/issues/627.
    // We can't assume that paths are always valid utf8 strings.
    let path = deno_fs::make_temp_dir(
      // Converting Option<String> to Option<&str>
      dir.as_ref().map(|x| &**x),
      prefix.as_ref().map(|x| &**x),
      suffix.as_ref().map(|x| &**x),
    )?;
    let builder = &mut FlatBufferBuilder::new();
    let path_off = builder.create_string(path.to_str().unwrap());
    let inner = msg::MakeTempDirRes::create(
      builder,
      &msg::MakeTempDirResArgs {
        path: Some(path_off),
        ..Default::default()
      },
    );
    Ok(serialize_response(
      cmd_id,
      builder,
      msg::BaseArgs {
        inner: Some(inner.as_union_value()),
        inner_type: msg::Any::MakeTempDirRes,
        ..Default::default()
      },
    ))
  })
}

fn op_mkdir(
  state: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  let inner = base.inner_as_mkdir().unwrap();
  let mode = inner.mode();
  let path = String::from(inner.path().unwrap());

  if !state.flags.allow_write {
    return odd_future(permission_denied());
  }

  blocking!(base.sync(), || {
    debug!("op_mkdir {}", path);
    deno_fs::mkdir(Path::new(&path), mode)?;
    Ok(empty_buf())
  })
}

fn op_open(
  _state: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  let cmd_id = base.cmd_id();
  let inner = base.inner_as_open().unwrap();
  let filename = PathBuf::from(inner.filename().unwrap());
  // TODO let perm = inner.perm();

  let op = tokio::fs::File::open(filename)
    .map_err(|err| DenoError::from(err))
    .and_then(move |fs_file| -> OpResult {
      let resource = resources::add_fs_file(fs_file);
      let builder = &mut FlatBufferBuilder::new();
      let inner = msg::OpenRes::create(
        builder,
        &msg::OpenResArgs {
          rid: resource.rid,
          ..Default::default()
        },
      );
      Ok(serialize_response(
        cmd_id,
        builder,
        msg::BaseArgs {
          inner: Some(inner.as_union_value()),
          inner_type: msg::Any::OpenRes,
          ..Default::default()
        },
      ))
    });
  Box::new(op)
}
