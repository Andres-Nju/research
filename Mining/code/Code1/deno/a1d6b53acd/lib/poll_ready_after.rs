  fn poll_ready(
    &mut self,
    _cx: &mut Context<'_>,
  ) -> Poll<Result<(), Self::Error>> {
    if self.inner.borrow().is_some() {
      Poll::Pending
    } else {
      Poll::Ready(Ok(()))
    }
  }

  fn call(&mut self, req: Request<Body>) -> Self::Future {
    let (resp_tx, resp_rx) = oneshot::channel();
    self.inner.borrow_mut().replace(ServiceInner {
      request: req,
      response_tx: resp_tx,
    });

    async move {
      resp_rx.await.or_else(|_|
        // Fallback dummy response in case sender was dropped due to closed conn
        Response::builder()
          .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
          .body(vec![].into()))
    }
    .boxed_local()
  }
}

type ConnFuture = Pin<Box<dyn Future<Output = hyper::Result<()>>>>;

struct Conn {
  scheme: &'static str,
  addr: SocketAddr,
  conn: Rc<RefCell<ConnFuture>>,
}

struct ConnResource {
  hyper_connection: Conn,
  deno_service: Service,
  cancel: CancelHandle,
}

impl ConnResource {
  // TODO(ry) impl Future for ConnResource?
  fn poll(&self, cx: &mut Context<'_>) -> Poll<Result<(), AnyError>> {
    self
      .hyper_connection
      .conn
      .borrow_mut()
      .poll_unpin(cx)
      .map_err(AnyError::from)
  }
}

impl Resource for ConnResource {
  fn name(&self) -> Cow<str> {
    "httpConnection".into()
  }

  fn close(self: Rc<Self>) {
    self.cancel.cancel()
  }
}

// We use a tuple instead of struct to avoid serialization overhead of the keys.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NextRequestResponse(
  // request_rid:
  Option<ResourceId>,
  // response_sender_rid:
  ResourceId,
  // method:
  // This is a String rather than a ByteString because reqwest will only return
  // the method as a str which is guaranteed to be ASCII-only.
  String,
  // headers:
  Vec<(ByteString, ByteString)>,
  // url:
  String,
);

async fn op_http_request_next(
  state: Rc<RefCell<OpState>>,
  conn_rid: ResourceId,
  _: (),
) -> Result<Option<NextRequestResponse>, AnyError> {
  let conn_resource = state
    .borrow()
    .resource_table
    .get::<ConnResource>(conn_rid)?;

  let cancel = RcRef::map(conn_resource.clone(), |r| &r.cancel);

  poll_fn(|cx| {
    conn_resource.deno_service.waker.register(cx.waker());

    // Check if conn is open/close/errored
    let (conn_closed, conn_result) = match conn_resource.poll(cx) {
      Poll::Pending => (false, Ok(())),
      Poll::Ready(Ok(())) => (true, Ok(())),
      Poll::Ready(Err(e)) => {
        if should_ignore_error(&e) {
          (true, Ok(()))
        } else {
          (true, Err(e))
        }
      }
    };
    // Drop conn resource if closed
    if conn_closed {
      // TODO(ry) close RequestResource associated with connection
      // TODO(ry) close ResponseBodyResource associated with connection
      // try to close ConnResource, but don't unwrap as it might
      // already be closed
      let _ = state
        .borrow_mut()
        .resource_table
        .take::<ConnResource>(conn_rid);

      // Fail with err if unexpected conn error, early return None otherwise
      return Poll::Ready(conn_result.map(|_| None));
    }

    if let Some(inner) = conn_resource.deno_service.inner.borrow_mut().take() {
      let Conn { scheme, addr, .. } = conn_resource.hyper_connection;
      let mut state = state.borrow_mut();
      let next =
        prepare_next_request(&mut state, conn_rid, inner, scheme, addr)?;
      Poll::Ready(Ok(Some(next)))
    } else {
      Poll::Pending
    }
  })
  .try_or_cancel(cancel)
  .await
  .map_err(AnyError::from)
}

fn prepare_next_request(
  state: &mut OpState,
  conn_rid: ResourceId,
  request_resource: ServiceInner,
  scheme: &'static str,
  addr: SocketAddr,
) -> Result<NextRequestResponse, AnyError> {
  let tx = request_resource.response_tx;
  let req = request_resource.request;
  let method = req.method().to_string();
  let headers = req_headers(&req);
  let url = req_url(&req, scheme, addr)?;

  let is_websocket = is_websocket_request(&req);
  let can_have_body = !matches!(*req.method(), Method::GET | Method::HEAD);
  let has_body =
    is_websocket || (can_have_body && req.size_hint().exact() != Some(0));

  let maybe_request_rid = if has_body {
    let request_rid = state.resource_table.add(RequestResource {
      conn_rid,
      inner: AsyncRefCell::new(RequestOrStreamReader::Request(Some(req))),
      cancel: CancelHandle::default(),
    });
    Some(request_rid)
  } else {
    None
  };

  let response_sender_rid = state.resource_table.add(ResponseSenderResource {
    sender: tx,
    conn_rid,
  });

  Ok(NextRequestResponse(
    maybe_request_rid,
    response_sender_rid,
    method,
    headers,
    url,
  ))
}

fn req_url(
  req: &hyper::Request<hyper::Body>,
  scheme: &'static str,
  addr: SocketAddr,
) -> Result<String, AnyError> {
  let host: Cow<str> = if let Some(auth) = req.uri().authority() {
    match addr.port() {
      443 if scheme == "https" => Cow::Borrowed(auth.host()),
      80 if scheme == "http" => Cow::Borrowed(auth.host()),
      _ => Cow::Borrowed(auth.as_str()), // Includes port number.
    }
  } else if let Some(host) = req.uri().host() {
    Cow::Borrowed(host)
  } else if let Some(host) = req.headers().get("HOST") {
    Cow::Borrowed(host.to_str()?)
  } else {
    Cow::Owned(addr.to_string())
  };
  let path = req.uri().path_and_query().map_or("/", |p| p.as_str());
  Ok([scheme, "://", &host, path].concat())
}

fn req_headers(
  req: &hyper::Request<hyper::Body>,
) -> Vec<(ByteString, ByteString)> {
  // We treat cookies specially, because we don't want them to get them
  // mangled by the `Headers` object in JS. What we do is take all cookie
  // headers and concat them into a single cookie header, separated by
  // semicolons.
  let cookie_sep = "; ".as_bytes();
  let mut cookies = vec![];

  let mut headers = Vec::with_capacity(req.headers().len());
  for (name, value) in req.headers().iter() {
    if name == hyper::header::COOKIE {
      cookies.push(value.as_bytes());
    } else {
      let name: &[u8] = name.as_ref();
      let value = value.as_bytes();
      headers.push((ByteString(name.to_owned()), ByteString(value.to_owned())));
    }
  }

  if !cookies.is_empty() {
    headers.push((
      ByteString("cookie".as_bytes().to_owned()),
      ByteString(cookies.join(cookie_sep)),
    ));
  }

  headers
}

fn is_websocket_request(req: &hyper::Request<hyper::Body>) -> bool {
  req.version() == hyper::Version::HTTP_11
    && req.method() == hyper::Method::GET
    && req.headers().contains_key(&SEC_WEBSOCKET_KEY)
    && header(req.headers(), &SEC_WEBSOCKET_VERSION) == b"13"
    && header(req.headers(), &UPGRADE).eq_ignore_ascii_case(b"websocket")
    && header(req.headers(), &CONNECTION)
      .split(|c| *c == b' ' || *c == b',')
      .any(|token| token.eq_ignore_ascii_case(b"upgrade"))
}

fn header<'a>(
  h: &'a hyper::http::HeaderMap,
  name: &hyper::header::HeaderName,
) -> &'a [u8] {
  h.get(name)
    .map(hyper::header::HeaderValue::as_bytes)
    .unwrap_or_default()
}

fn should_ignore_error(e: &AnyError) -> bool {
  if let Some(e) = e.downcast_ref::<hyper::Error>() {
    use std::error::Error;
    if let Some(std_err) = e.source() {
      if let Some(io_err) = std_err.downcast_ref::<std::io::Error>() {
        if io_err.kind() == std::io::ErrorKind::NotConnected {
          return true;
        }
      }
    }
  }
  false
}

pub fn start_http<IO: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
  state: &mut OpState,
  io: IO,
  addr: SocketAddr,
  scheme: &'static str,
) -> Result<ResourceId, AnyError> {
  let deno_service = Service::default();

  let hyper_connection = Http::new()
    .with_executor(LocalExecutor)
    .serve_connection(io, deno_service.clone())
    .with_upgrades();
  let conn = Pin::new(Box::new(hyper_connection));
  let conn_resource = ConnResource {
    hyper_connection: Conn {
      scheme,
      addr,
      conn: Rc::new(RefCell::new(conn)),
    },
    deno_service,
    cancel: CancelHandle::default(),
  };
  let rid = state.resource_table.add(conn_resource);
  Ok(rid)
}

// We use a tuple instead of struct to avoid serialization overhead of the keys.
#[derive(Deserialize)]
struct RespondArgs(
  // rid:
  u32,
  // status:
  u16,
  // headers:
  Vec<(ByteString, ByteString)>,
);

async fn op_http_response(
  state: Rc<RefCell<OpState>>,
  args: RespondArgs,
  data: Option<ZeroCopyBuf>,
) -> Result<Option<ResourceId>, AnyError> {
  let RespondArgs(rid, status, headers) = args;

  let response_sender = state
    .borrow_mut()
    .resource_table
    .take::<ResponseSenderResource>(rid)?;
  let response_sender = Rc::try_unwrap(response_sender)
    .ok()
    .expect("multiple op_http_respond ongoing");

  let conn_rid = response_sender.conn_rid;

  let conn_resource = state
    .borrow()
    .resource_table
    .get::<ConnResource>(conn_rid)?;

  let mut builder = Response::builder().status(status);

  builder.headers_mut().unwrap().reserve(headers.len());
  for (key, value) in &headers {
    builder = builder.header(key.as_ref(), value.as_ref());
  }

  let res;
  let maybe_response_body_rid = if let Some(d) = data {
    // If a body is passed, we use it, and don't return a body for streaming.
    res = builder.body(Vec::from(&*d).into())?;
    None
  } else {
    // If no body is passed, we return a writer for streaming the body.
    let (sender, body) = Body::channel();
    res = builder.body(body)?;

    let response_body_rid =
      state.borrow_mut().resource_table.add(ResponseBodyResource {
        body: AsyncRefCell::new(sender),
        conn_rid,
      });

    Some(response_body_rid)
  };
