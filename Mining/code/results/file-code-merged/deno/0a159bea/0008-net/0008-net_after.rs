// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use crate::ops::io::TcpStreamResource;
use crate::permissions::Permissions;
use crate::resolve_addr::resolve_addr;
use crate::resolve_addr::resolve_addr_sync;
use deno_core::error::bad_resource;
use deno_core::error::bad_resource_id;
use deno_core::error::custom_error;
use deno_core::error::generic_error;
use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::serde_json;
use deno_core::serde_json::json;
use deno_core::serde_json::Value;
use deno_core::AsyncRefCell;
use deno_core::BufVec;
use deno_core::CancelHandle;
use deno_core::CancelTryFuture;
use deno_core::OpState;
use deno_core::RcRef;
use deno_core::Resource;
use deno_core::ZeroCopyBuf;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::net::UdpSocket;
use trust_dns_proto::rr::record_data::RData;
use trust_dns_proto::rr::record_type::RecordType;
use trust_dns_resolver::config::NameServerConfigGroup;
use trust_dns_resolver::config::ResolverConfig;
use trust_dns_resolver::config::ResolverOpts;
use trust_dns_resolver::system_conf;
use trust_dns_resolver::AsyncResolver;

#[cfg(unix)]
use super::net_unix;
#[cfg(unix)]
use crate::ops::io::StreamResource;
#[cfg(unix)]
use std::path::Path;

pub fn init(rt: &mut deno_core::JsRuntime) {
  super::reg_json_async(rt, "op_accept", op_accept);
  super::reg_json_async(rt, "op_connect", op_connect);
  super::reg_json_async(rt, "op_shutdown", op_shutdown);
  super::reg_json_sync(rt, "op_listen", op_listen);
  super::reg_json_async(rt, "op_datagram_receive", op_datagram_receive);
  super::reg_json_async(rt, "op_datagram_send", op_datagram_send);
  super::reg_json_async(rt, "op_dns_resolve", op_dns_resolve);
}

#[derive(Deserialize)]
pub(crate) struct AcceptArgs {
  pub rid: i32,
  pub transport: String,
}

async fn accept_tcp(
  state: Rc<RefCell<OpState>>,
  args: AcceptArgs,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let rid = args.rid as u32;

  let resource = state
    .borrow()
    .resource_table
    .get::<TcpListenerResource>(rid)
    .ok_or_else(|| bad_resource("Listener has been closed"))?;
  let listener = RcRef::map(&resource, |r| &r.listener)
    .try_borrow_mut()
    .ok_or_else(|| custom_error("Busy", "Another accept task is ongoing"))?;
  let cancel = RcRef::map(resource, |r| &r.cancel);
  let (tcp_stream, _socket_addr) =
    listener.accept().try_or_cancel(cancel).await.map_err(|e| {
      // FIXME(bartlomieju): compatibility with current JS implementation
      if let std::io::ErrorKind::Interrupted = e.kind() {
        bad_resource("Listener has been closed")
      } else {
        e.into()
      }
    })?;
  let local_addr = tcp_stream.local_addr()?;
  let remote_addr = tcp_stream.peer_addr()?;

  let mut state = state.borrow_mut();
  let rid = state
    .resource_table
    .add(TcpStreamResource::new(tcp_stream.into_split()));
  Ok(json!({
    "rid": rid,
    "localAddr": {
      "hostname": local_addr.ip().to_string(),
      "port": local_addr.port(),
      "transport": "tcp",
    },
    "remoteAddr": {
      "hostname": remote_addr.ip().to_string(),
      "port": remote_addr.port(),
      "transport": "tcp",
    }
  }))
}

async fn op_accept(
  state: Rc<RefCell<OpState>>,
  args: Value,
  bufs: BufVec,
) -> Result<Value, AnyError> {
  let args: AcceptArgs = serde_json::from_value(args)?;
  match args.transport.as_str() {
    "tcp" => accept_tcp(state, args, bufs).await,
    #[cfg(unix)]
    "unix" => net_unix::accept_unix(state, args, bufs).await,
    _ => Err(generic_error(format!(
      "Unsupported transport protocol {}",
      args.transport
    ))),
  }
}

#[derive(Deserialize)]
pub(crate) struct ReceiveArgs {
  pub rid: i32,
  pub transport: String,
}

async fn receive_udp(
  state: Rc<RefCell<OpState>>,
  args: ReceiveArgs,
  zero_copy: BufVec,
) -> Result<Value, AnyError> {
  assert_eq!(zero_copy.len(), 1, "Invalid number of arguments");
  let mut zero_copy = zero_copy[0].clone();

  let rid = args.rid as u32;

  let resource = state
    .borrow_mut()
    .resource_table
    .get::<UdpSocketResource>(rid)
    .ok_or_else(|| bad_resource("Socket has been closed"))?;
  let socket = RcRef::map(&resource, |r| &r.socket).borrow().await;
  let cancel_handle = RcRef::map(&resource, |r| &r.cancel);
  let (size, remote_addr) = socket
    .recv_from(&mut zero_copy)
    .try_or_cancel(cancel_handle)
    .await?;
  Ok(json!({
    "size": size,
    "remoteAddr": {
      "hostname": remote_addr.ip().to_string(),
      "port": remote_addr.port(),
      "transport": "udp",
    }
  }))
}

async fn op_datagram_receive(
  state: Rc<RefCell<OpState>>,
  args: Value,
  zero_copy: BufVec,
) -> Result<Value, AnyError> {
  assert_eq!(zero_copy.len(), 1, "Invalid number of arguments");

  let args: ReceiveArgs = serde_json::from_value(args)?;
  match args.transport.as_str() {
    "udp" => receive_udp(state, args, zero_copy).await,
    #[cfg(unix)]
    "unixpacket" => net_unix::receive_unix_packet(state, args, zero_copy).await,
    _ => Err(generic_error(format!(
      "Unsupported transport protocol {}",
      args.transport
    ))),
  }
}

#[derive(Deserialize)]
struct SendArgs {
  rid: i32,
  transport: String,
  #[serde(flatten)]
  transport_args: ArgsEnum,
}

async fn op_datagram_send(
  state: Rc<RefCell<OpState>>,
  args: Value,
  zero_copy: BufVec,
) -> Result<Value, AnyError> {
  assert_eq!(zero_copy.len(), 1, "Invalid number of arguments");
  let zero_copy = zero_copy[0].clone();

  match serde_json::from_value(args)? {
    SendArgs {
      rid,
      transport,
      transport_args: ArgsEnum::Ip(args),
    } if transport == "udp" => {
      {
        let s = state.borrow();
        s.borrow::<Permissions>()
          .check_net(&(&args.hostname, Some(args.port)))?;
      }
      let addr = resolve_addr(&args.hostname, args.port)
        .await?
        .next()
        .ok_or_else(|| generic_error("No resolved address found"))?;

      let resource = state
        .borrow_mut()
        .resource_table
        .get::<UdpSocketResource>(rid as u32)
        .ok_or_else(|| bad_resource("Socket has been closed"))?;
      let socket = RcRef::map(&resource, |r| &r.socket).borrow().await;
      let byte_length = socket.send_to(&zero_copy, &addr).await?;
      Ok(json!(byte_length))
    }
    #[cfg(unix)]
    SendArgs {
      rid,
      transport,
      transport_args: ArgsEnum::Unix(args),
    } if transport == "unixpacket" => {
      let address_path = Path::new(&args.path);
      {
        let s = state.borrow();
        s.borrow::<Permissions>().check_write(&address_path)?;
      }
      let resource = state
        .borrow()
        .resource_table
        .get::<net_unix::UnixDatagramResource>(rid as u32)
        .ok_or_else(|| {
          custom_error("NotConnected", "Socket has been closed")
        })?;
      let socket = RcRef::map(&resource, |r| &r.socket)
        .try_borrow_mut()
        .ok_or_else(|| custom_error("Busy", "Socket already in use"))?;
      let byte_length = socket.send_to(&zero_copy, address_path).await?;
      Ok(json!(byte_length))
    }
    _ => Err(type_error("Wrong argument format!")),
  }
}

#[derive(Deserialize)]
struct ConnectArgs {
  transport: String,
  #[serde(flatten)]
  transport_args: ArgsEnum,
}

async fn op_connect(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  match serde_json::from_value(args)? {
    ConnectArgs {
      transport,
      transport_args: ArgsEnum::Ip(args),
    } if transport == "tcp" => {
      {
        let state_ = state.borrow();
        state_
          .borrow::<Permissions>()
          .check_net(&(&args.hostname, Some(args.port)))?;
      }
      let addr = resolve_addr(&args.hostname, args.port)
        .await?
        .next()
        .ok_or_else(|| generic_error("No resolved address found"))?;
      let tcp_stream = TcpStream::connect(&addr).await?;
      let local_addr = tcp_stream.local_addr()?;
      let remote_addr = tcp_stream.peer_addr()?;

      let mut state_ = state.borrow_mut();
      let rid = state_
        .resource_table
        .add(TcpStreamResource::new(tcp_stream.into_split()));
      Ok(json!({
        "rid": rid,
        "localAddr": {
          "hostname": local_addr.ip().to_string(),
          "port": local_addr.port(),
          "transport": transport,
        },
        "remoteAddr": {
          "hostname": remote_addr.ip().to_string(),
          "port": remote_addr.port(),
          "transport": transport,
        }
      }))
    }
    #[cfg(unix)]
    ConnectArgs {
      transport,
      transport_args: ArgsEnum::Unix(args),
    } if transport == "unix" => {
      let address_path = Path::new(&args.path);
      super::check_unstable2(&state, "Deno.connect");
      {
        let state_ = state.borrow();
        state_.borrow::<Permissions>().check_read(&address_path)?;
        state_.borrow::<Permissions>().check_write(&address_path)?;
      }
      let path = args.path;
      let unix_stream = net_unix::UnixStream::connect(Path::new(&path)).await?;
      let local_addr = unix_stream.local_addr()?;
      let remote_addr = unix_stream.peer_addr()?;

      let mut state_ = state.borrow_mut();
      let resource = StreamResource::unix_stream(unix_stream);
      let rid = state_.resource_table.add(resource);
      Ok(json!({
        "rid": rid,
        "localAddr": {
          "path": local_addr.as_pathname(),
          "transport": transport,
        },
        "remoteAddr": {
          "path": remote_addr.as_pathname(),
          "transport": transport,
        }
      }))
    }
    _ => Err(type_error("Wrong argument format!")),
  }
}

#[derive(Deserialize)]
struct ShutdownArgs {
  rid: i32,
}

async fn op_shutdown(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: ShutdownArgs = serde_json::from_value(args)?;
  let rid = args.rid as u32;

  let resource = state
    .borrow()
    .resource_table
    .get_any(rid)
    .ok_or_else(bad_resource_id)?;
  if let Some(stream) = resource.downcast_rc::<TcpStreamResource>() {
    let mut wr = stream.wr_borrow_mut().await;
    wr.shutdown().await?;
    return Ok(json!({}));
  }

  #[cfg(unix)]
  if let Some(stream) = resource.downcast_rc::<StreamResource>() {
    if stream.unix_stream.is_some() {
      let mut wr = RcRef::map(stream, |r| r.unix_stream.as_ref().unwrap())
        .borrow_mut()
        .await;
      wr.shutdown().await?;
      return Ok(json!({}));
    }
  }

  Err(bad_resource_id())
}

struct TcpListenerResource {
  listener: AsyncRefCell<TcpListener>,
  cancel: CancelHandle,
}

impl Resource for TcpListenerResource {
  fn name(&self) -> Cow<str> {
    "tcpListener".into()
  }

  fn close(self: Rc<Self>) {
    self.cancel.cancel();
  }
}

struct UdpSocketResource {
  socket: AsyncRefCell<UdpSocket>,
  cancel: CancelHandle,
}

impl Resource for UdpSocketResource {
  fn name(&self) -> Cow<str> {
    "udpSocket".into()
  }

  fn close(self: Rc<Self>) {
    self.cancel.cancel()
  }
}

#[derive(Deserialize)]
struct IpListenArgs {
  hostname: String,
  port: u16,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ArgsEnum {
  Ip(IpListenArgs),
  #[cfg(unix)]
  Unix(net_unix::UnixListenArgs),
}

#[derive(Deserialize)]
struct ListenArgs {
  transport: String,
  #[serde(flatten)]
  transport_args: ArgsEnum,
}

fn listen_tcp(
  state: &mut OpState,
  addr: SocketAddr,
) -> Result<(u32, SocketAddr), AnyError> {
  let std_listener = std::net::TcpListener::bind(&addr)?;
  std_listener.set_nonblocking(true)?;
  let listener = TcpListener::from_std(std_listener)?;
  let local_addr = listener.local_addr()?;
  let listener_resource = TcpListenerResource {
    listener: AsyncRefCell::new(listener),
    cancel: Default::default(),
  };
  let rid = state.resource_table.add(listener_resource);

  Ok((rid, local_addr))
}

fn listen_udp(
  state: &mut OpState,
  addr: SocketAddr,
) -> Result<(u32, SocketAddr), AnyError> {
  let std_socket = std::net::UdpSocket::bind(&addr)?;
  std_socket.set_nonblocking(true)?;
  let socket = UdpSocket::from_std(std_socket)?;
  let local_addr = socket.local_addr()?;
  let socket_resource = UdpSocketResource {
    socket: AsyncRefCell::new(socket),
    cancel: Default::default(),
  };
  let rid = state.resource_table.add(socket_resource);

  Ok((rid, local_addr))
}

fn op_listen(
  state: &mut OpState,
  args: Value,
  _zero_copy: &mut [ZeroCopyBuf],
) -> Result<Value, AnyError> {
  let permissions = state.borrow::<Permissions>();
  match serde_json::from_value(args)? {
    ListenArgs {
      transport,
      transport_args: ArgsEnum::Ip(args),
    } => {
      {
        if transport == "udp" {
          super::check_unstable(state, "Deno.listenDatagram");
        }
        permissions.check_net(&(&args.hostname, Some(args.port)))?;
      }
      let addr = resolve_addr_sync(&args.hostname, args.port)?
        .next()
        .ok_or_else(|| generic_error("No resolved address found"))?;
      let (rid, local_addr) = if transport == "tcp" {
        listen_tcp(state, addr)?
      } else {
        listen_udp(state, addr)?
      };
      debug!(
        "New listener {} {}:{}",
        rid,
        local_addr.ip().to_string(),
        local_addr.port()
      );
      Ok(json!({
      "rid": rid,
      "localAddr": {
        "hostname": local_addr.ip().to_string(),
        "port": local_addr.port(),
        "transport": transport,
      },
      }))
    }
    #[cfg(unix)]
    ListenArgs {
      transport,
      transport_args: ArgsEnum::Unix(args),
    } if transport == "unix" || transport == "unixpacket" => {
      let address_path = Path::new(&args.path);
      {
        if transport == "unix" {
          super::check_unstable(state, "Deno.listen");
        }
        if transport == "unixpacket" {
          super::check_unstable(state, "Deno.listenDatagram");
        }
        permissions.check_read(&address_path)?;
        permissions.check_write(&address_path)?;
      }
      let (rid, local_addr) = if transport == "unix" {
        net_unix::listen_unix(state, &address_path)?
      } else {
        net_unix::listen_unix_packet(state, &address_path)?
      };
      debug!(
        "New listener {} {}",
        rid,
        local_addr.as_pathname().unwrap().display(),
      );
      Ok(json!({
      "rid": rid,
      "localAddr": {
        "path": local_addr.as_pathname(),
        "transport": transport,
      },
      }))
    }
    #[cfg(unix)]
    _ => Err(type_error("Wrong argument format!")),
  }
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(untagged)]
enum DnsReturnRecord {
  A(String),
  AAAA(String),
  ANAME(String),
  CNAME(String),
  MX {
    preference: u16,
    exchange: String,
  },
  PTR(String),
  SRV {
    priority: u16,
    weight: u16,
    port: u16,
    target: String,
  },
  TXT(Vec<String>),
}

async fn op_dns_resolve(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  fn default_port() -> u16 {
    53
  }

  #[derive(Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct ResolveAddrArgs {
    query: String,
    record_type: RecordType,
    options: Option<ResolveDnsOption>,
  }

  #[derive(Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct ResolveDnsOption {
    name_server: Option<NameServer>,
  }

  #[derive(Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct NameServer {
    ip_addr: String,
    #[serde(default = "default_port")]
    port: u16,
  }

  let ResolveAddrArgs {
    query,
    record_type,
    options,
  } = serde_json::from_value(args)?;

  let (config, opts) = if let Some(name_server) =
    options.as_ref().and_then(|o| o.name_server.as_ref())
  {
    let group = NameServerConfigGroup::from_ips_clear(
      &[name_server.ip_addr.parse()?],
      name_server.port,
      true,
    );
    (
      ResolverConfig::from_parts(None, vec![], group),
      ResolverOpts::default(),
    )
  } else {
    system_conf::read_system_conf()?
  };

  {
    let s = state.borrow();
    let perm = s.borrow::<Permissions>();

    // Checks permission against the name servers which will be actually queried.
    for ns in config.name_servers() {
      let socker_addr = &ns.socket_addr;
      let ip = socker_addr.ip().to_string();
      let port = socker_addr.port();
      perm.check_net(&(ip, Some(port)))?;
    }
  }

  let resolver = AsyncResolver::tokio(config, opts)?;

  let results: Vec<DnsReturnRecord> = resolver
    .lookup(query, record_type, Default::default())
    .await
    .map_err(|e| generic_error(format!("{}", e)))?
    .iter()
    .filter_map(rdata_to_return_record(record_type))
    .collect();

  Ok(json!(results))
}

fn rdata_to_return_record(
  ty: RecordType,
) -> impl Fn(&RData) -> Option<DnsReturnRecord> {
  use RecordType::*;
  move |r: &RData| -> Option<DnsReturnRecord> {
    match ty {
      A => r.as_a().map(ToString::to_string).map(DnsReturnRecord::A),
      AAAA => r
        .as_aaaa()
        .map(ToString::to_string)
        .map(DnsReturnRecord::AAAA),
      ANAME => r
        .as_aname()
        .map(ToString::to_string)
        .map(DnsReturnRecord::ANAME),
      CNAME => r
        .as_cname()
        .map(ToString::to_string)
        .map(DnsReturnRecord::CNAME),
      MX => r.as_mx().map(|mx| DnsReturnRecord::MX {
        preference: mx.preference(),
        exchange: mx.exchange().to_string(),
      }),
      PTR => r
        .as_ptr()
        .map(ToString::to_string)
        .map(DnsReturnRecord::PTR),
      SRV => r.as_srv().map(|srv| DnsReturnRecord::SRV {
        priority: srv.priority(),
        weight: srv.weight(),
        port: srv.port(),
        target: srv.target().to_string(),
      }),
      TXT => r.as_txt().map(|txt| {
        let texts: Vec<String> = txt
          .iter()
          .map(|bytes| {
            // Tries to parse these bytes as Latin-1
            bytes.iter().map(|&b| b as char).collect::<String>()
          })
          .collect();
        DnsReturnRecord::TXT(texts)
      }),
      // TODO(magurotuna): Other record types are not supported
      _ => todo!(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::net::Ipv4Addr;
  use std::net::Ipv6Addr;
  use trust_dns_proto::rr::rdata::mx::MX;
  use trust_dns_proto::rr::rdata::srv::SRV;
  use trust_dns_proto::rr::rdata::txt::TXT;
  use trust_dns_proto::rr::record_data::RData;
  use trust_dns_proto::rr::Name;

  #[test]
  fn rdata_to_return_record_a() {
    let func = rdata_to_return_record(RecordType::A);
    let rdata = RData::A(Ipv4Addr::new(127, 0, 0, 1));
    assert_eq!(
      func(&rdata),
      Some(DnsReturnRecord::A("127.0.0.1".to_string()))
    );
  }

  #[test]
  fn rdata_to_return_record_aaaa() {
    let func = rdata_to_return_record(RecordType::AAAA);
    let rdata = RData::AAAA(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
    assert_eq!(func(&rdata), Some(DnsReturnRecord::AAAA("::1".to_string())));
  }

  #[test]
  fn rdata_to_return_record_aname() {
    let func = rdata_to_return_record(RecordType::ANAME);
    let rdata = RData::ANAME(Name::new());
    assert_eq!(func(&rdata), Some(DnsReturnRecord::ANAME("".to_string())));
  }

  #[test]
  fn rdata_to_return_record_cname() {
    let func = rdata_to_return_record(RecordType::CNAME);
    let rdata = RData::CNAME(Name::new());
    assert_eq!(func(&rdata), Some(DnsReturnRecord::CNAME("".to_string())));
  }

  #[test]
  fn rdata_to_return_record_mx() {
    let func = rdata_to_return_record(RecordType::MX);
    let rdata = RData::MX(MX::new(10, Name::new()));
    assert_eq!(
      func(&rdata),
      Some(DnsReturnRecord::MX {
        preference: 10,
        exchange: "".to_string()
      })
    );
  }

  #[test]
  fn rdata_to_return_record_ptr() {
    let func = rdata_to_return_record(RecordType::PTR);
    let rdata = RData::PTR(Name::new());
    assert_eq!(func(&rdata), Some(DnsReturnRecord::PTR("".to_string())));
  }

  #[test]
  fn rdata_to_return_record_srv() {
    let func = rdata_to_return_record(RecordType::SRV);
    let rdata = RData::SRV(SRV::new(1, 2, 3, Name::new()));
    assert_eq!(
      func(&rdata),
      Some(DnsReturnRecord::SRV {
        priority: 1,
        weight: 2,
        port: 3,
        target: "".to_string()
      })
    );
  }

  #[test]
  fn rdata_to_return_record_txt() {
    let func = rdata_to_return_record(RecordType::TXT);
    let rdata = RData::TXT(TXT::from_bytes(vec![
      "foo".as_bytes(),
      "bar".as_bytes(),
      &[0xa3],             // "£" in Latin-1
      &[0xe3, 0x81, 0x82], // "あ" in UTF-8
    ]));
    assert_eq!(
      func(&rdata),
      Some(DnsReturnRecord::TXT(vec![
        "foo".to_string(),
        "bar".to_string(),
        "£".to_string(),
        "ã\u{81}\u{82}".to_string(),
      ]))
    );
  }
}
