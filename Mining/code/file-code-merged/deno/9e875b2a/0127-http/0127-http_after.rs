// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use crate::ops::io::TcpStreamResource;
use crate::ops::io::TlsStreamResource;
use crate::ops::tls::TlsStream;
use deno_core::error::bad_resource_id;
use deno_core::error::null_opbuf;
use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::futures::future::poll_fn;
use deno_core::futures::FutureExt;
use deno_core::futures::Stream;
use deno_core::futures::StreamExt;
use deno_core::op_async;
use deno_core::op_sync;
use deno_core::AsyncRefCell;
use deno_core::ByteString;
use deno_core::CancelHandle;
use deno_core::CancelTryFuture;
use deno_core::Extension;
use deno_core::OpState;
use deno_core::RcRef;
use deno_core::Resource;
use deno_core::ResourceId;
use deno_core::ZeroCopyBuf;
use hyper::body::HttpBody;
use hyper::http;
use hyper::server::conn::Connection;
use hyper::server::conn::Http;
use hyper::service::Service as HyperService;
use hyper::Body;
use hyper::Request;
use hyper::Response;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;
use std::cell::RefCell;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::task::Poll;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio_util::io::StreamReader;

pub fn init() -> Extension {
  Extension::builder()
    .ops(vec![
      ("op_http_start", op_sync(op_http_start)),
      ("op_http_request_next", op_async(op_http_request_next)),
      ("op_http_request_read", op_async(op_http_request_read)),
      ("op_http_response", op_async(op_http_response)),
      ("op_http_response_write", op_async(op_http_response_write)),
      ("op_http_response_close", op_async(op_http_response_close)),
    ])
    .build()
}

struct ServiceInner {
  request: Request<Body>,
  response_tx: oneshot::Sender<Response<Body>>,
}

#[derive(Clone, Default)]
struct Service {
  inner: Rc<RefCell<Option<ServiceInner>>>,
  waker: Rc<deno_core::futures::task::AtomicWaker>,
}

impl HyperService<Request<Body>> for Service {
  type Response = Response<Body>;
  type Error = http::Error;
  #[allow(clippy::type_complexity)]
  type Future =
    Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

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

    async move { Ok(resp_rx.await.unwrap()) }.boxed_local()
  }
}

enum ConnType {
  Tcp(Rc<RefCell<Connection<TcpStream, Service, LocalExecutor>>>),
  Tls(Rc<RefCell<Connection<TlsStream, Service, LocalExecutor>>>),
}

struct ConnResource {
  hyper_connection: ConnType,
  deno_service: Service,
  addr: SocketAddr,
  cancel: CancelHandle,
}

impl ConnResource {
  // TODO(ry) impl Future for ConnResource?
  fn poll(&self, cx: &mut Context<'_>) -> Poll<Result<(), AnyError>> {
    match &self.hyper_connection {
      ConnType::Tcp(c) => c.borrow_mut().poll_unpin(cx),
      ConnType::Tls(c) => c.borrow_mut().poll_unpin(cx),
    }
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
  // request_body_rid:
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
    .get::<ConnResource>(conn_rid)
    .ok_or_else(bad_resource_id)?;

  let cancel = RcRef::map(conn_resource.clone(), |r| &r.cancel);

  poll_fn(|cx| {
    conn_resource.deno_service.waker.register(cx.waker());
    let connection_closed = match conn_resource.poll(cx) {
      Poll::Pending => false,
      Poll::Ready(Ok(())) => {
        // try to close ConnResource, but don't unwrap as it might
        // already be closed
        let _ = state
          .borrow_mut()
          .resource_table
          .take::<ConnResource>(conn_rid);
        true
      }
      Poll::Ready(Err(e)) => {
        // TODO(ry) close RequestResource associated with connection
        // TODO(ry) close ResponseBodyResource associated with connection
        // close ConnResource
        state
          .borrow_mut()
          .resource_table
          .take::<ConnResource>(conn_rid)
          .unwrap();

        if should_ignore_error(&e) {
          true
        } else {
          return Poll::Ready(Err(e));
        }
      }
    };
    if let Some(request_resource) =
      conn_resource.deno_service.inner.borrow_mut().take()
    {
      let tx = request_resource.response_tx;
      let req = request_resource.request;
      let method = req.method().to_string();

      let mut headers = Vec::with_capacity(req.headers().len());
      for (name, value) in req.headers().iter() {
        let name: &[u8] = name.as_ref();
        let value = value.as_bytes();
        headers
          .push((ByteString(name.to_owned()), ByteString(value.to_owned())));
      }

      let url = {
        let scheme = {
          match conn_resource.hyper_connection {
            ConnType::Tcp(_) => "http",
            ConnType::Tls(_) => "https",
          }
        };
        let host: Cow<str> = if let Some(host) = req.uri().host() {
          Cow::Borrowed(host)
        } else if let Some(host) = req.headers().get("HOST") {
          Cow::Borrowed(host.to_str()?)
        } else {
          Cow::Owned(conn_resource.addr.to_string())
        };
        let path = req.uri().path_and_query().map_or("/", |p| p.as_str());
        format!("{}://{}{}", scheme, host, path)
      };

      let has_body = if let Some(exact_size) = req.size_hint().exact() {
        exact_size > 0
      } else {
        true
      };

      let maybe_request_body_rid = if has_body {
        let stream: BytesStream = Box::pin(req.into_body().map(|r| {
          r.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
        }));
        let stream_reader = StreamReader::new(stream);
        let mut state = state.borrow_mut();
        let request_body_rid = state.resource_table.add(RequestBodyResource {
          conn_rid,
          reader: AsyncRefCell::new(stream_reader),
          cancel: CancelHandle::default(),
        });
        Some(request_body_rid)
      } else {
        None
      };

      let mut state = state.borrow_mut();
      let response_sender_rid =
        state.resource_table.add(ResponseSenderResource {
          sender: tx,
          conn_rid,
        });

      Poll::Ready(Ok(Some(NextRequestResponse(
        maybe_request_body_rid,
        response_sender_rid,
        method,
        headers,
        url,
      ))))
    } else if connection_closed {
      Poll::Ready(Ok(None))
    } else {
      Poll::Pending
    }
  })
  .try_or_cancel(cancel)
  .await
  .map_err(AnyError::from)
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

fn op_http_start(
  state: &mut OpState,
  tcp_stream_rid: ResourceId,
  _: (),
) -> Result<ResourceId, AnyError> {
  let deno_service = Service::default();

  if let Some(resource_rc) = state
    .resource_table
    .take::<TcpStreamResource>(tcp_stream_rid)
  {
    let resource = Rc::try_unwrap(resource_rc)
      .expect("Only a single use of this resource should happen");
    let (read_half, write_half) = resource.into_inner();
    let tcp_stream = read_half.reunite(write_half)?;
    let addr = tcp_stream.local_addr()?;
    let hyper_connection = Http::new()
      .with_executor(LocalExecutor)
      .serve_connection(tcp_stream, deno_service.clone());
    let conn_resource = ConnResource {
      hyper_connection: ConnType::Tcp(Rc::new(RefCell::new(hyper_connection))),
      deno_service,
      addr,
      cancel: CancelHandle::default(),
    };
    let rid = state.resource_table.add(conn_resource);
    return Ok(rid);
  }

  if let Some(resource_rc) = state
    .resource_table
    .take::<TlsStreamResource>(tcp_stream_rid)
  {
    let resource = Rc::try_unwrap(resource_rc)
      .expect("Only a single use of this resource should happen");
    let (read_half, write_half) = resource.into_inner();
    let tls_stream = read_half.reunite(write_half);
    let addr = tls_stream.get_ref().0.local_addr()?;

    let hyper_connection = Http::new()
      .with_executor(LocalExecutor)
      .serve_connection(tls_stream, deno_service.clone());
    let conn_resource = ConnResource {
      hyper_connection: ConnType::Tls(Rc::new(RefCell::new(hyper_connection))),
      deno_service,
      addr,
      cancel: CancelHandle::default(),
    };
    let rid = state.resource_table.add(conn_resource);
    return Ok(rid);
  }

  Err(bad_resource_id())
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
    .take::<ResponseSenderResource>(rid)
    .ok_or_else(bad_resource_id)?;
  let response_sender = Rc::try_unwrap(response_sender)
    .ok()
    .expect("multiple op_http_respond ongoing");

  let conn_resource = state
    .borrow()
    .resource_table
    .get::<ConnResource>(response_sender.conn_rid)
    .ok_or_else(bad_resource_id)?;

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
        conn_rid: response_sender.conn_rid,
      });

    Some(response_body_rid)
  };

  // oneshot::Sender::send(v) returns |v| on error, not an error object.
  // The only failure mode is the receiver already having dropped its end
  // of the channel.
  if response_sender.sender.send(res).is_err() {
    return Err(type_error("internal communication error"));
  }

  poll_fn(|cx| match conn_resource.poll(cx) {
    Poll::Ready(x) => Poll::Ready(x),
    Poll::Pending => Poll::Ready(Ok(())),
  })
  .await?;

  if maybe_response_body_rid.is_none() {
    conn_resource.deno_service.waker.wake();
  }
  Ok(maybe_response_body_rid)
}

async fn op_http_response_close(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  _: (),
) -> Result<(), AnyError> {
  let resource = state
    .borrow_mut()
    .resource_table
    .take::<ResponseBodyResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let conn_resource = state
    .borrow()
    .resource_table
    .get::<ConnResource>(resource.conn_rid)
    .ok_or_else(bad_resource_id)?;
  drop(resource);

  let r = poll_fn(|cx| match conn_resource.poll(cx) {
    Poll::Ready(x) => Poll::Ready(x),
    Poll::Pending => Poll::Ready(Ok(())),
  })
  .await;
  conn_resource.deno_service.waker.wake();
  r
}

async fn op_http_request_read(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  data: Option<ZeroCopyBuf>,
) -> Result<usize, AnyError> {
  let mut data = data.ok_or_else(null_opbuf)?;

  let resource = state
    .borrow()
    .resource_table
    .get::<RequestBodyResource>(rid as u32)
    .ok_or_else(bad_resource_id)?;

  let conn_resource = state
    .borrow()
    .resource_table
    .get::<ConnResource>(resource.conn_rid)
    .ok_or_else(bad_resource_id)?;

  let mut reader = RcRef::map(&resource, |r| &r.reader).borrow_mut().await;
  let cancel = RcRef::map(resource, |r| &r.cancel);
  let mut read_fut = reader.read(&mut data).try_or_cancel(cancel).boxed_local();

  poll_fn(|cx| {
    if let Poll::Ready(Err(e)) = conn_resource.poll(cx) {
      // close ConnResource
      // close RequestResource associated with connection
      // close ResponseBodyResource associated with connection
      return Poll::Ready(Err(e));
    }

    read_fut.poll_unpin(cx).map_err(AnyError::from)
  })
  .await
}

async fn op_http_response_write(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  data: Option<ZeroCopyBuf>,
) -> Result<(), AnyError> {
  let buf = data.ok_or_else(null_opbuf)?;
  let resource = state
    .borrow()
    .resource_table
    .get::<ResponseBodyResource>(rid as u32)
    .ok_or_else(bad_resource_id)?;

  let conn_resource = state
    .borrow()
    .resource_table
    .get::<ConnResource>(resource.conn_rid)
    .ok_or_else(bad_resource_id)?;

  let mut body = RcRef::map(&resource, |r| &r.body).borrow_mut().await;

  let mut send_data_fut = body.send_data(Vec::from(&*buf).into()).boxed_local();

  poll_fn(|cx| {
    let r = send_data_fut.poll_unpin(cx).map_err(AnyError::from);

    // Poll connection so the data is flushed
    if let Poll::Ready(Err(e)) = conn_resource.poll(cx) {
      // close ConnResource
      // close RequestResource associated with connection
      // close ResponseBodyResource associated with connection
      return Poll::Ready(Err(e));
    }

    r
  })
  .await?;

  Ok(())
}

type BytesStream =
  Pin<Box<dyn Stream<Item = std::io::Result<bytes::Bytes>> + Unpin>>;

struct RequestBodyResource {
  conn_rid: ResourceId,
  reader: AsyncRefCell<StreamReader<BytesStream, bytes::Bytes>>,
  cancel: CancelHandle,
}

impl Resource for RequestBodyResource {
  fn name(&self) -> Cow<str> {
    "requestBody".into()
  }

  fn close(self: Rc<Self>) {
    self.cancel.cancel()
  }
}

struct ResponseSenderResource {
  sender: oneshot::Sender<Response<Body>>,
  conn_rid: ResourceId,
}

impl Resource for ResponseSenderResource {
  fn name(&self) -> Cow<str> {
    "responseSender".into()
  }
}

struct ResponseBodyResource {
  body: AsyncRefCell<hyper::body::Sender>,
  conn_rid: ResourceId,
}

impl Resource for ResponseBodyResource {
  fn name(&self) -> Cow<str> {
    "responseBody".into()
  }
}

// Needed so hyper can use non Send futures
#[derive(Clone)]
struct LocalExecutor;

impl<Fut> hyper::rt::Executor<Fut> for LocalExecutor
where
  Fut: Future + 'static,
  Fut::Output: 'static,
{
  fn execute(&self, fut: Fut) {
    tokio::task::spawn_local(fut);
  }
}
