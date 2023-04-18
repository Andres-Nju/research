    pub fn retransmit(obj: &Arc<RwLock<Self>>, blob: &SharedBlob, s: &UdpSocket) -> Result<()> {
        let (me, table): (NodeInfo, Vec<NodeInfo>) = {
            // copy to avoid locking during IO
            let s = obj.read().expect("'obj' read lock in pub fn retransmit");
            (s.table[&s.me].clone(), s.table.values().cloned().collect())
        };
        blob.write()
            .unwrap()
            .set_id(me.id)
            .expect("set_id in pub fn retransmit");
        let rblob = blob.read().unwrap();
        let orders: Vec<_> = table
            .iter()
            .filter(|v| {
                if me.id == v.id {
                    false
                } else if me.leader_id == v.id {
                    trace!("skip retransmit to leader {:?}", v.id);
                    false
                } else if !(Self::is_valid_address(v.contact_info.tvu)) {
                    trace!("skip nodes that are not listening {:?}", v.id);
                    false
                } else {
                    true
                }
            })
            .collect();
        trace!("retransmit orders {}", orders.len());
        let errs: Vec<_> = orders
            .par_iter()
            .map(|v| {
                debug!(
                    "{:x}: retransmit blob {} to {:x}",
                    me.debug_id(),
                    rblob.get_index().unwrap(),
                    v.debug_id(),
                );
                //TODO profile this, may need multiple sockets for par_iter
                assert!(rblob.meta.size < BLOB_SIZE);
                s.send_to(&rblob.data[..rblob.meta.size], &v.contact_info.tvu)
            })
            .collect();
        for e in errs {
            if let Err(e) = &e {
                inc_new_counter!("crdt-retransmit-send_to_error", 1, 1);
                error!("retransmit result {:?}", e);
            }
            e?;
        }
        Ok(())
    }
