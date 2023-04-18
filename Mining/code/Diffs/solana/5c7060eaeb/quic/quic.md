File_Code/solana/5c7060eaeb/quic/quic_after.rs --- Rust
571         let s = UdpSocket::bind("127.0.0.1:0").unwrap();                                                                                                   . 
572         let exit = Arc::new(AtomicBool::new(false));                                                                                                       . 
573         let (sender, receiver) = unbounded();                                                                                                              . 
574         let keypair = Keypair::new();                                                                                                                      . 
575         let ip = "127.0.0.1".parse().unwrap();                                                                                                             . 
576         let server_address = s.local_addr().unwrap();                                                                                                      . 
577         let t = spawn_server(s, &keypair, ip, sender, exit.clone(), 1).unwrap();                                                                         571         let (t, exit, receiver, server_address) = setup_quic_server();

