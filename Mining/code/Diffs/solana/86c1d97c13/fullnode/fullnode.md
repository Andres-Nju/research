File_Code/solana/86c1d97c13/fullnode/fullnode_after.rs --- Rust
300         let mut rpc_addr = node.data.contact_info.ncp;                                                                                                   300         let rpc_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), RPC_PORT);
301         rpc_addr.set_port(RPC_PORT);                                                                                                                         

