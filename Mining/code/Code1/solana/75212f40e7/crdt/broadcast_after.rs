    pub fn broadcast(
        me: &NodeInfo,
        broadcast_table: &[NodeInfo],
        window: &Window,
        s: &UdpSocket,
        transmit_index: &mut u64,
        received_index: u64,
    ) -> Result<()> {
        if broadcast_table.is_empty() {
            warn!("{:x}:not enough peers in crdt table", me.debug_id());
            inc_new_counter!("crdt-broadcast-not_enough_peers_error", 1);
            Err(CrdtError::NoPeers)?;
        }
        trace!("broadcast nodes {}", broadcast_table.len());

        // enumerate all the blobs in the window, those are the indices
        // transmit them to nodes, starting from a different node
        let mut orders = Vec::new();
        let window_l = window.write().unwrap();
        let mut br_idx = *transmit_index as usize % broadcast_table.len();

        for idx in *transmit_index..received_index {
            let w_idx = idx as usize % window_l.len();
            assert!(window_l[w_idx].data.is_some());

            orders.push((window_l[w_idx].data.clone(), &broadcast_table[br_idx]));

            br_idx += 1;
            br_idx %= broadcast_table.len();

            if window_l[w_idx].coding.is_some() {
                orders.push((window_l[w_idx].coding.clone(), &broadcast_table[br_idx]));
                br_idx += 1;
                br_idx %= broadcast_table.len();
            }
        }

        trace!("broadcast orders table {}", orders.len());
        let errs: Vec<_> = orders
            .into_iter()
            .map(|(b, v)| {
                // only leader should be broadcasting
                assert!(me.leader_id != v.id);
                let bl = b.unwrap();
                let blob = bl.read().expect("blob read lock in streamer::broadcast");
                //TODO profile this, may need multiple sockets for par_iter
                trace!(
                    "{:x}: BROADCAST idx: {} sz: {} to {:x},{} coding: {}",
                    me.debug_id(),
                    blob.get_index().unwrap(),
                    blob.meta.size,
                    v.debug_id(),
                    v.contact_info.tvu,
                    blob.is_coding()
                );
                assert!(blob.meta.size <= BLOB_SIZE);
                let e = s.send_to(&blob.data[..blob.meta.size], &v.contact_info.tvu);
                trace!(
                    "{:x}: done broadcast {} to {:x} {}",
                    me.debug_id(),
                    blob.meta.size,
                    v.debug_id(),
                    v.contact_info.tvu
                );
                e
            })
            .collect();
        trace!("broadcast results {}", errs.len());
        for e in errs {
            if let Err(e) = &e {
                error!("broadcast result {:?}", e);
            }
            e?;
            *transmit_index += 1;
        }
        Ok(())
    }
