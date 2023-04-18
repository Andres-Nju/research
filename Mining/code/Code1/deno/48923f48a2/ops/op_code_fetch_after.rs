fn op_code_fetch(
  state: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  let inner = base.inner_as_code_fetch().unwrap();
  let cmd_id = base.cmd_id();
  let module_specifier = inner.module_specifier().unwrap();
  let containing_file = inner.containing_file().unwrap();

  assert_eq!(state.dir.root.join("gen"), state.dir.gen, "Sanity check");

  Box::new(futures::future::result(|| -> OpResult {
    let builder = &mut FlatBufferBuilder::new();
    let out = state.dir.code_fetch(module_specifier, containing_file)?;
    let mut msg_args = msg::CodeFetchResArgs {
      module_name: Some(builder.create_string(&out.module_name)),
      filename: Some(builder.create_string(&out.filename)),
      source_code: Some(builder.create_string(&out.source_code)),
      ..Default::default()
    };
    match out.maybe_output_code {
      Some(ref output_code) => {
        msg_args.output_code = Some(builder.create_string(output_code));
      }
      _ => (),
    };
    let inner = msg::CodeFetchRes::create(builder, &msg_args);
    Ok(serialize_response(
      cmd_id,
      builder,
      msg::BaseArgs {
        inner: Some(inner.as_union_value()),
        inner_type: msg::Any::CodeFetchRes,
        ..Default::default()
      },
    ))
  }()))
}

// https://github.com/denoland/deno/blob/golang/os.go#L156-L169
fn op_code_cache(
  state: Arc<IsolateState>,
  base: &msg::Base,
  data: &'static mut [u8],
) -> Box<Op> {
  assert_eq!(data.len(), 0);
  let inner = base.inner_as_code_cache().unwrap();
  let filename = inner.filename().unwrap();
  let source_code = inner.source_code().unwrap();
  let output_code = inner.output_code().unwrap();
  Box::new(futures::future::result(|| -> OpResult {
    state.dir.code_cache(filename, source_code, output_code)?;
    Ok(empty_buf())
  }()))
}
