fn producer(addr: &SocketAddr, exit: Arc<AtomicBool>) -> JoinHandle<()> {
    let send = UdpSocket::bind("0.0.0.0:0").unwrap();
    let mut msgs = Packets::default();
    msgs.packets.resize(10, Packet::default());
    for w in msgs.packets.iter_mut() {
        w.meta.size = PACKET_DATA_SIZE;
        w.meta.set_addr(&addr);
    }
    let msgs = Arc::new(msgs);
    spawn(move || loop {
        if exit.load(Ordering::Relaxed) {
            return;
        }
        let mut num = 0;
        for p in &msgs.packets {
            let a = p.meta.addr();
            assert!(p.meta.size < PACKET_DATA_SIZE);
            send.send_to(&p.data[..p.meta.size], &a).unwrap();
            num += 1;
        }
        assert_eq!(num, 10);
    })
}
