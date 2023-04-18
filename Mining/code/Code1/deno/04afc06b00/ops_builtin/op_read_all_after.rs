async fn op_read_all(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
) -> Result<ZeroCopyBuf, Error> {
  let resource = state.borrow().resource_table.get_any(rid)?;

  // The number of bytes we attempt to grow the buffer by each time it fills
  // up and we have more data to read. We start at 64 KB. The grow_len is
  // doubled if the nread returned from a single read is equal or greater than
  // the grow_len. This allows us to reduce allocations for resources that can
  // read large chunks of data at a time.
  let mut grow_len: usize = 64 * 1024;

  let (min, maybe_max) = resource.size_hint();
  // Try to determine an optimial starting buffer size for this resource based
  // on the size hint.
  let initial_size = match (min, maybe_max) {
    (min, Some(max)) if min == max => min as usize,
    (_min, Some(max)) if (max as usize) < grow_len => max as usize,
    (min, _) if (min as usize) < grow_len => grow_len,
    (min, _) => min as usize,
  };

  let mut buf = BufMutView::new(initial_size);
  loop {
    // if the buffer does not have much remaining space, we may have to grow it.
    if buf.len() < grow_len {
      let vec = buf.get_mut_vec();
      match maybe_max {
        Some(max) if vec.len() >= max as usize => {
          // no need to resize the vec, because the vec is already large enough
          // to accommodate the maximum size of the read data.
        }
        Some(max) if (max as usize) < vec.len() + grow_len => {
          // grow the vec to the maximum size of the read data
          vec.resize(max as usize, 0);
        }
        _ => {
          // grow the vec by grow_len
          vec.resize(vec.len() + grow_len, 0);
        }
      }
    }
    let (n, new_buf) = resource.clone().read_byob(buf).await?;
    buf = new_buf;
    buf.advance_cursor(n);
    if n == 0 {
      break;
    }
    if n >= grow_len {
      // we managed to read more or equal data than fits in a single grow_len in
      // a single go, so let's attempt to read even more next time. this reduces
      // allocations for resources that can read large chunks of data at a time.
      grow_len *= 2;
    }
  }

  let nread = buf.reset_cursor();
  let mut vec = buf.unwrap_vec();
  // If the buffer is larger than the amount of data read, shrink it to the
  // amount of data read.
  if nread < vec.len() {
    vec.truncate(nread);
  }

  Ok(ZeroCopyBuf::from(vec))
}
