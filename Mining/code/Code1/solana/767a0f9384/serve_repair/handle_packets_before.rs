    fn handle_packets(
        me: &Arc<RwLock<Self>>,
        recycler: &PacketsRecycler,
        blockstore: Option<&Arc<Blockstore>>,
        packets: Packets,
        response_sender: &PacketSender,
        stats: &mut ServeRepairStats,
    ) {
        // iter over the packets, collect pulls separately and process everything else
        let allocated = thread_mem_usage::Allocatedp::default();
        packets.packets.iter().for_each(|packet| {
            let start = allocated.get();
            let from_addr = packet.meta.addr();
            limited_deserialize(&packet.data[..packet.meta.size])
                .into_iter()
                .for_each(|request| {
                    stats.processed += 1;
                    let rsp =
                        Self::handle_repair(me, recycler, &from_addr, blockstore, request, stats);
                    if let Some(rsp) = rsp {
                        let _ignore_disconnect = response_sender.send(rsp);
                    }
                });
            datapoint_debug!(
                "solana-serve-repair-memory",
                ("serve_repair", (allocated.get() - start) as i64, i64),
            );
        });
    }
