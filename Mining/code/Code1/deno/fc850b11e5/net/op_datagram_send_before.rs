fn op_datagram_send(
  isolate_state: &mut CoreIsolateState,
  state: &State,
  args: Value,
  zero_copy: &mut [ZeroCopyBuf],
) -> Result<JsonOp, OpError> {
  assert_eq!(zero_copy.len(), 1, "Invalid number of arguments");
  let zero_copy = zero_copy[0].clone();

  let resource_table = isolate_state.resource_table.clone();
  match serde_json::from_value(args)? {
    SendArgs {
      rid,
      transport,
      transport_args: ArgsEnum::Ip(args),
    } if transport == "udp" => {
      state.check_net(&args.hostname, args.port)?;
      let addr = resolve_addr(&args.hostname, args.port)?;
      let f = poll_fn(move |cx| {
        let mut resource_table = resource_table.borrow_mut();
        let resource = resource_table
          .get_mut::<UdpSocketResource>(rid as u32)
          .ok_or_else(|| {
            OpError::bad_resource("Socket has been closed".to_string())
          })?;
        resource
          .socket
          .poll_send_to(cx, &zero_copy, &addr)
          .map_err(OpError::from)
          .map_ok(|byte_length| json!(byte_length))
      });
      Ok(JsonOp::Async(f.boxed_local()))
    }
    #[cfg(unix)]
    SendArgs {
      rid,
      transport,
      transport_args: ArgsEnum::Unix(args),
    } if transport == "unixpacket" => {
      let address_path = net_unix::Path::new(&args.path);
      state.check_read(&address_path)?;
      let op = async move {
        let mut resource_table = resource_table.borrow_mut();
        let resource = resource_table
          .get_mut::<net_unix::UnixDatagramResource>(rid as u32)
          .ok_or_else(|| {
            OpError::other("Socket has been closed".to_string())
          })?;

        let socket = &mut resource.socket;
        socket
          .send_to(&zero_copy, &resource.local_addr.as_pathname().unwrap())
          .await?;

        Ok(json!({}))
      };

      Ok(JsonOp::Async(op.boxed_local()))
    }
    _ => Err(OpError::other("Wrong argument format!".to_owned())),
  }
}
