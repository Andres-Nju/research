    fn test_get_public_ip_addr() {
        solana_logger::setup();
        let (_server_port, (server_udp_socket, server_tcp_listener)) =
            bind_common_in_range((3200, 3250)).unwrap();
        let (client_port, (client_udp_socket, client_tcp_listener)) =
            bind_common_in_range((3200, 3250)).unwrap();

        let _runtime = ip_echo_server(server_tcp_listener);

        let ip_echo_server_addr = server_udp_socket.local_addr().unwrap();
        get_public_ip_addr(&ip_echo_server_addr).unwrap();

        verify_reachable_ports(
            &ip_echo_server_addr,
            vec![(client_port, client_tcp_listener)],
            &[&client_udp_socket],
        );
    }
