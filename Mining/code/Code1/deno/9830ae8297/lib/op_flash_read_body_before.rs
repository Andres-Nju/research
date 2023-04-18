async fn op_flash_read_body(
  state: Rc<RefCell<OpState>>,
  server_id: u32,
  token: u32,
  mut buf: ZeroCopyBuf,
) -> usize {
  // SAFETY: we cannot hold op_state borrow across the await point. The JS caller
  // is responsible for ensuring this is not called concurrently.
  let ctx = unsafe {
    {
      let op_state = &mut state.borrow_mut();
      let flash_ctx = op_state.borrow_mut::<FlashContext>();
      flash_ctx.servers.get_mut(&server_id).unwrap() as *mut ServerContext
    }
    .as_mut()
    .unwrap()
  };
  let tx = ctx.requests.get_mut(&token).unwrap();

  if tx.te_chunked {
    let mut decoder =
      chunked::Decoder::new(tx.socket(), tx.remaining_chunk_size);
    loop {
      let sock = tx.socket();

      let _lock = sock.read_lock.lock().unwrap();
      match decoder.read(&mut buf) {
        Ok(n) => {
          tx.remaining_chunk_size = decoder.remaining_chunks_size;
          return n;
        }
        Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
          panic!("chunked read error: {}", e);
        }
        Err(_) => {
          drop(_lock);
          sock.read_rx.as_mut().unwrap().recv().await.unwrap();
        }
      }
    }
  }

  if let Some(content_length) = tx.content_length {
    let sock = tx.socket();
    let l = sock.read_lock.clone();

    loop {
      let _lock = l.lock().unwrap();
      if tx.content_read >= content_length as usize {
        return 0;
      }
      match sock.read(&mut buf) {
        Ok(n) => {
          tx.content_read += n;
          return n;
        }
        _ => {
          drop(_lock);
          sock.read_rx.as_mut().unwrap().recv().await.unwrap();
        }
      }
    }
  }

  0
}
