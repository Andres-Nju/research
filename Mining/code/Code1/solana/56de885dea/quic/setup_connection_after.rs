async fn setup_connection(
    connecting: Connecting,
    unstaked_connection_table: Arc<Mutex<ConnectionTable>>,
    staked_connection_table: Arc<Mutex<ConnectionTable>>,
    packet_sender: Sender<PacketBatch>,
    max_connections_per_peer: usize,
    staked_nodes: Arc<RwLock<StakedNodes>>,
    max_staked_connections: usize,
    max_unstaked_connections: usize,
    stats: Arc<StreamStats>,
) {
    if let Ok(connecting_result) = timeout(
        Duration::from_millis(QUIC_CONNECTION_HANDSHAKE_TIMEOUT_MS),
        connecting,
    )
    .await
    {
        if let Ok(new_connection) = connecting_result {
            stats.total_new_connections.fetch_add(1, Ordering::Relaxed);

            let params = get_connection_stake(&new_connection.connection, staked_nodes.clone())
                .map_or(
                    NewConnectionHandlerParams::new_unstaked(
                        packet_sender.clone(),
                        max_connections_per_peer,
                        stats.clone(),
                    ),
                    |(pubkey, stake, total_stake, max_stake, min_stake)| {
                        NewConnectionHandlerParams {
                            packet_sender,
                            remote_pubkey: Some(pubkey),
                            stake,
                            total_stake,
                            max_connections_per_peer,
                            stats: stats.clone(),
                            max_stake,
                            min_stake,
                        }
                    },
                );

            if params.stake > 0 {
                let mut connection_table_l = staked_connection_table.lock().unwrap();
                if connection_table_l.total_size >= max_staked_connections {
                    let num_pruned = connection_table_l.prune_random(params.stake);
                    stats.num_evictions.fetch_add(num_pruned, Ordering::Relaxed);
                }

                if connection_table_l.total_size < max_staked_connections {
                    if let Ok(()) = handle_and_cache_new_connection(
                        new_connection,
                        connection_table_l,
                        staked_connection_table.clone(),
                        &params,
                    ) {
                        stats
                            .connection_added_from_staked_peer
                            .fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    // If we couldn't prune a connection in the staked connection table, let's
                    // put this connection in the unstaked connection table. If needed, prune a
                    // connection from the unstaked connection table.
                    if let Ok(()) = prune_unstaked_connections_and_add_new_connection(
                        new_connection,
                        unstaked_connection_table.lock().unwrap(),
                        unstaked_connection_table.clone(),
                        max_unstaked_connections,
                        &params,
                    ) {
                        stats
                            .connection_added_from_staked_peer
                            .fetch_add(1, Ordering::Relaxed);
                    } else {
                        stats
                            .connection_add_failed_on_pruning
                            .fetch_add(1, Ordering::Relaxed);
                        stats
                            .connection_add_failed_staked_node
                            .fetch_add(1, Ordering::Relaxed);
                    }
                }
            } else if let Ok(()) = prune_unstaked_connections_and_add_new_connection(
                new_connection,
                unstaked_connection_table.lock().unwrap(),
                unstaked_connection_table.clone(),
                max_unstaked_connections,
                &params,
            ) {
                stats
                    .connection_added_from_unstaked_peer
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                stats
                    .connection_add_failed_unstaked_node
                    .fetch_add(1, Ordering::Relaxed);
            }
        } else {
            stats.connection_setup_error.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        stats
            .connection_setup_timeout
            .fetch_add(1, Ordering::Relaxed);
    }
}

