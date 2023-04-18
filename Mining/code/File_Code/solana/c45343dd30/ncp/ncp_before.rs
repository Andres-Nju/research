//! The `ncp` module implements the network control plane.

use crdt;
use packet;
use result::Result;
use std::net::UdpSocket;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use streamer;

pub struct Ncp {
    pub thread_hdls: Vec<JoinHandle<()>>,
}

impl Ncp {
    pub fn new(
        crdt: Arc<RwLock<crdt::Crdt>>,
        window: Arc<RwLock<Vec<Option<packet::SharedBlob>>>>,
        gossip_listen_socket: UdpSocket,
        gossip_send_socket: UdpSocket,
        exit: Arc<AtomicBool>,
    ) -> Result<Ncp> {
        let blob_recycler = packet::BlobRecycler::default();
        let (request_sender, request_receiver) = channel();
        trace!(
            "Ncp: id: {:?}, listening on: {:?}",
            &crdt.read().unwrap().me[..4],
            gossip_listen_socket.local_addr().unwrap()
        );
        let t_receiver = streamer::blob_receiver(
            exit.clone(),
            blob_recycler.clone(),
            gossip_listen_socket,
            request_sender,
        )?;
        let (response_sender, response_receiver) = channel();
        let t_responder = streamer::responder(
            gossip_send_socket,
            exit.clone(),
            blob_recycler.clone(),
            response_receiver,
        );
        let t_listen = crdt::Crdt::listen(
            crdt.clone(),
            window,
            blob_recycler.clone(),
            request_receiver,
            response_sender.clone(),
            exit.clone(),
        );
        let t_gossip = crdt::Crdt::gossip(crdt.clone(), blob_recycler, response_sender, exit);
        let thread_hdls = vec![t_receiver, t_responder, t_listen, t_gossip];
        Ok(Ncp { thread_hdls })
    }
}

#[cfg(test)]
mod tests {
    use crdt::{Crdt, TestNode};
    use ncp::Ncp;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, RwLock};

    #[test]
    #[ignore]
    // test that stage will exit when flag is set
    fn test_exit() {
        let exit = Arc::new(AtomicBool::new(false));
        let tn = TestNode::new();
        let crdt = Crdt::new(tn.data.clone());
        let c = Arc::new(RwLock::new(crdt));
        let w = Arc::new(RwLock::new(vec![]));
        let d = Ncp::new(
            c.clone(),
            w,
            tn.sockets.gossip,
            tn.sockets.gossip_send,
            exit.clone(),
        ).unwrap();
        exit.store(true, Ordering::Relaxed);
        for t in d.thread_hdls {
            t.join().expect("thread join");
        }
    }
}
