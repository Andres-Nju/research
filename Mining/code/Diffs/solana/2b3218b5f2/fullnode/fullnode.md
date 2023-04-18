File_Code/solana/2b3218b5f2/fullnode/fullnode_after.rs --- 1/2 --- Rust
11 use crate::gossip_service::GossipService;                                                                                                                 11 use crate::gossip_service::{discover_nodes, GossipService};

File_Code/solana/2b3218b5f2/fullnode/fullnode_after.rs --- 2/2 --- Rust
398                                                                                                                                                          398     discover_nodes(&contact_info.gossip, 1).expect("Node startup failed");

