fn retransmit(
    bank_forks: &RwLock<BankForks>,
    leader_schedule_cache: &LeaderScheduleCache,
    cluster_info: &ClusterInfo,
    r: &Mutex<PacketReceiver>,
    sock: &UdpSocket,
    id: u32,
    stats: &RetransmitStats,
    cluster_nodes: &RwLock<ClusterNodes<RetransmitStage>>,
    last_peer_update: &AtomicU64,
    shreds_received: &Mutex<ShredFilterAndHasher>,
    max_slots: &MaxSlots,
    first_shreds_received: &Mutex<BTreeSet<Slot>>,
    rpc_subscriptions: &Option<Arc<RpcSubscriptions>>,
) -> Result<()> {
    const RECV_TIMEOUT: Duration = Duration::from_secs(1);
    let r_lock = r.lock().unwrap();
    let packets = r_lock.recv_timeout(RECV_TIMEOUT)?;
    let mut timer_start = Measure::start("retransmit");
    let mut total_packets = packets.packets.len();
    let mut packets = vec![packets];
    while let Ok(nq) = r_lock.try_recv() {
        total_packets += nq.packets.len();
        packets.push(nq);
        if total_packets >= MAX_PACKET_BATCH_SIZE {
            break;
        }
    }
    drop(r_lock);

    let mut epoch_fetch = Measure::start("retransmit_epoch_fetch");
    let (working_bank, root_bank) = {
        let bank_forks = bank_forks.read().unwrap();
        (bank_forks.working_bank(), bank_forks.root_bank())
    };
    let bank_epoch = working_bank.get_leader_schedule_epoch(working_bank.slot());
    epoch_fetch.stop();

    let mut epoch_cache_update = Measure::start("retransmit_epoch_cach_update");
    maybe_update_peers_cache(
        cluster_nodes,
        shreds_received,
        last_peer_update,
        cluster_info,
        bank_epoch,
        &working_bank,
    );
    let cluster_nodes = cluster_nodes.read().unwrap();
    let mut peers_len = 0;
    epoch_cache_update.stop();

    let my_id = cluster_info.id();
    let socket_addr_space = cluster_info.socket_addr_space();
    let mut discard_total = 0;
    let mut repair_total = 0;
    let mut retransmit_total = 0;
    let mut compute_turbine_peers_total = 0;
    let mut retransmit_tree_mismatch = 0;
    let mut packets_by_slot: HashMap<Slot, usize> = HashMap::new();
    let mut packets_by_source: HashMap<String, usize> = HashMap::new();
    let mut max_slot = 0;
    for packet in packets.iter().flat_map(|p| p.packets.iter()) {
        // skip discarded packets and repair packets
        if packet.meta.discard {
            total_packets -= 1;
            discard_total += 1;
            continue;
        }
        if packet.meta.repair {
            total_packets -= 1;
            repair_total += 1;
            continue;
        }
        let shred_slot = match check_if_already_received(packet, shreds_received) {
            Some(slot) => slot,
            None => continue,
        };
        max_slot = max_slot.max(shred_slot);

        if let Some(rpc_subscriptions) = rpc_subscriptions {
            if check_if_first_shred_received(shred_slot, first_shreds_received, &root_bank) {
                rpc_subscriptions.notify_slot_update(SlotUpdate::FirstShredReceived {
                    slot: shred_slot,
                    timestamp: timestamp(),
                });
            }
        }

        let mut compute_turbine_peers = Measure::start("turbine_start");
        let slot_leader = leader_schedule_cache.slot_leader_at(shred_slot, Some(&working_bank));
        let (neighbors, children) =
            cluster_nodes.get_retransmit_peers(packet.meta.seed, DATA_PLANE_FANOUT, slot_leader);
        // If the node is on the critical path (i.e. the first node in each
        // neighborhood), then we expect that the packet arrives at tvu socket
        // as opposed to tvu-forwards. If this is not the case, then the
        // turbine broadcast/retransmit tree is mismatched across nodes.
        let anchor_node = neighbors[0].id == my_id;
        if packet.meta.forward == anchor_node {
            // TODO: Consider forwarding the packet to the root node here.
            retransmit_tree_mismatch += 1;
        }
        peers_len = peers_len.max(cluster_nodes.num_peers());
        compute_turbine_peers.stop();
        compute_turbine_peers_total += compute_turbine_peers.as_us();

        *packets_by_slot.entry(packet.meta.slot).or_default() += 1;
        *packets_by_source
            .entry(packet.meta.addr().to_string())
            .or_default() += 1;

        let mut retransmit_time = Measure::start("retransmit_to");
        // If the node is on the critical path (i.e. the first node in each
        // neighborhood), it should send the packet to tvu socket of its
        // children and also tvu_forward socket of its neighbors. Otherwise it
        // should only forward to tvu_forward socket of its children.
        if anchor_node {
            // First neighbor is this node itself, so skip it.
            ClusterInfo::retransmit_to(
                &neighbors[1..],
                packet,
                sock,
                /*forward socket=*/ true,
                socket_addr_space,
            );
        }
        ClusterInfo::retransmit_to(
            &children,
            packet,
            sock,
            !anchor_node, // send to forward socket!
            socket_addr_space,
        );
        retransmit_time.stop();
        retransmit_total += retransmit_time.as_us();
    }
    max_slots.retransmit.fetch_max(max_slot, Ordering::Relaxed);
    timer_start.stop();
    debug!(
        "retransmitted {} packets in {}ms retransmit_time: {}ms id: {}",
        total_packets,
        timer_start.as_ms(),
        retransmit_total,
        id,
    );
    update_retransmit_stats(
        stats,
        timer_start.as_us(),
        total_packets,
        retransmit_total,
        discard_total,
        repair_total,
        compute_turbine_peers_total,
        peers_len,
        packets_by_slot,
        packets_by_source,
        epoch_fetch.as_us(),
        epoch_cache_update.as_us(),
        retransmit_tree_mismatch,
    );

    Ok(())
}
