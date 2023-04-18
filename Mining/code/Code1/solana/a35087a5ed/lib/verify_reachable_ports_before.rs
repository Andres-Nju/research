pub fn verify_reachable_ports(
    ip_echo_server_addr: &SocketAddr,
    tcp_listeners: Vec<(u16, TcpListener)>,
    udp_sockets: &[&UdpSocket],
) {
    let udp: Vec<(_, _)> = udp_sockets
        .iter()
        .map(|udp_socket| {
            (
                udp_socket.local_addr().unwrap().port(),
                udp_socket.try_clone().expect("Unable to clone udp socket"),
            )
        })
        .collect();

    let udp_ports: Vec<_> = udp.iter().map(|x| x.0).collect();

    info!(
        "Checking that tcp ports {:?} and udp ports {:?} are reachable from {:?}",
        tcp_listeners, udp_ports, ip_echo_server_addr
    );

    let tcp_ports: Vec<_> = tcp_listeners.iter().map(|(port, _)| *port).collect();
    let _ = ip_echo_server_request(
        ip_echo_server_addr,
        IpEchoServerMessage::new(&tcp_ports, &udp_ports),
    )
    .map_err(|err| warn!("ip_echo_server request failed: {}", err));

    // Wait for a connection to open on each TCP port
    for (port, tcp_listener) in tcp_listeners {
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            debug!("Waiting for incoming connection on tcp/{}", port);
            let _ = tcp_listener.incoming().next().expect("tcp incoming failed");
            sender.send(()).expect("send failure");
        });
        receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|err| {
                error!(
                    "Received no response at tcp/{}, check your port configuration: {}",
                    port, err
                );
                std::process::exit(1);
            });
        info!("tdp/{} is reachable", port);
    }

    // Wait for a datagram to arrive at each UDP port
    for (port, udp_socket) in udp {
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let mut buf = [0; 1];
            debug!("Waiting for incoming datagram on udp/{}", port);
            let _ = udp_socket.recv(&mut buf).expect("udp recv failure");
            sender.send(()).expect("send failure");
        });
        receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|err| {
                error!(
                    "Received no response at udp/{}, check your port configuration: {}",
                    port, err
                );
                std::process::exit(1);
            });
        info!("udp/{} is reachable", port);
    }
}
