fn recv_window<F>(
    blocktree: &Arc<Blocktree>,
    my_pubkey: &Pubkey,
    r: &PacketReceiver,
    retransmit: &PacketSender,
    shred_filter: F,
) -> Result<()>
where
    F: Fn(&Shred) -> bool,
    F: Sync,
{
    let timer = Duration::from_millis(200);
    let mut packets = r.recv_timeout(timer)?;

    while let Ok(mut more_packets) = r.try_recv() {
        packets.packets.append(&mut more_packets.packets)
    }
    let now = Instant::now();
    inc_new_counter_debug!("streamer-recv_window-recv", packets.packets.len());

    let mut shreds = vec![];
    let mut discards = vec![];
    for (i, packet) in packets.packets.iter_mut().enumerate() {
        if let Ok(s) = bincode::deserialize(&packet.data) {
            let shred: Shred = s;
            if shred_filter(&shred) {
                packet.meta.slot = shred.slot();
                packet.meta.seed = shred.seed();
                shreds.push(shred);
            } else {
                discards.push(i);
            }
        } else {
            discards.push(i);
        }
    }

    for i in discards.into_iter().rev() {
        packets.packets.remove(i);
    }

    trace!("{:?} shreds from packets", shreds.len());

    trace!(
        "{} num shreds received: {}",
        my_pubkey,
        packets.packets.len()
    );

    if !packets.packets.is_empty() {
        match retransmit.send(packets) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }?;
    }

    blocktree.insert_shreds(shreds)?;

    trace!(
        "Elapsed processing time in recv_window(): {}",
        duration_as_ms(&now.elapsed())
    );

    Ok(())
}
