async fn handle_connection(
    mut uni_streams: IncomingUniStreams,
    packet_sender: Sender<PacketBatch>,
    remote_addr: SocketAddr,
    remote_pubkey: Option<Pubkey>,
    last_update: Arc<AtomicU64>,
    connection_table: Arc<Mutex<ConnectionTable>>,
    stream_exit: Arc<AtomicBool>,
    stats: Arc<StreamStats>,
    stake: u64,
    peer_type: ConnectionPeerType,
) {
    debug!(
        "quic new connection {} streams: {} connections: {}",
        remote_addr,
        stats.total_streams.load(Ordering::Relaxed),
        stats.total_connections.load(Ordering::Relaxed),
    );
    stats.total_connections.fetch_add(1, Ordering::Relaxed);
    while !stream_exit.load(Ordering::Relaxed) {
        if let Ok(stream) = tokio::time::timeout(
            Duration::from_millis(WAIT_FOR_STREAM_TIMEOUT_MS),
            uni_streams.next(),
        )
        .await
        {
            match stream {
                Some(stream_result) => match stream_result {
                    Ok(mut stream) => {
                        stats.total_streams.fetch_add(1, Ordering::Relaxed);
                        stats.total_new_streams.fetch_add(1, Ordering::Relaxed);
                        let stream_exit = stream_exit.clone();
                        let stats = stats.clone();
                        let packet_sender = packet_sender.clone();
                        let last_update = last_update.clone();
                        tokio::spawn(async move {
                            let mut maybe_batch = None;
                            while !stream_exit.load(Ordering::Relaxed) {
                                if let Ok(chunk) = tokio::time::timeout(
                                    Duration::from_millis(WAIT_FOR_STREAM_TIMEOUT_MS),
                                    stream.read_chunk(PACKET_DATA_SIZE, false),
                                )
                                .await
                                {
                                    if handle_chunk(
                                        &chunk,
                                        &mut maybe_batch,
                                        &remote_addr,
                                        &packet_sender,
                                        stats.clone(),
                                        stake,
                                        peer_type,
                                    ) {
                                        last_update.store(timing::timestamp(), Ordering::Relaxed);
                                        break;
                                    }
                                } else {
                                    debug!("Timeout in receiving on stream");
                                    stats
                                        .total_stream_read_timeouts
                                        .fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            stats.total_streams.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) => {
                        debug!("stream error: {:?}", e);
                        break;
                    }
                },
                None => {
                    break;
                }
            }
        }
    }

    if connection_table.lock().unwrap().remove_connection(
        ConnectionTableKey::new(remote_addr.ip(), remote_pubkey),
        remote_addr.port(),
    ) {
        stats.connection_removed.fetch_add(1, Ordering::Relaxed);
    } else {
        stats
            .connection_remove_failed
            .fetch_add(1, Ordering::Relaxed);
    }
    stats.total_connections.fetch_sub(1, Ordering::Relaxed);
}
