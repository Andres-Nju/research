    fn transmit(
        &mut self,
        receiver: &Arc<Mutex<TransmitReceiver>>,
        cluster_info: &ClusterInfo,
        sock: &UdpSocket,
        bank_forks: &Arc<RwLock<BankForks>>,
    ) -> Result<()> {
        let (shreds, _) = receiver.lock().unwrap().recv()?;
        if shreds.is_empty() {
            return Ok(());
        }
        let slot = shreds.first().unwrap().slot();
        assert!(shreds.iter().all(|shred| shred.slot() == slot));
        let (root_bank, working_bank) = {
            let bank_forks = bank_forks.read().unwrap();
            (bank_forks.root_bank(), bank_forks.working_bank())
        };
        let self_pubkey = cluster_info.id();
        let nodes: Vec<_> = cluster_info
            .all_peers()
            .into_iter()
            .map(|(node, _)| node)
            .collect();

        // Creat cluster partition.
        let cluster_partition: HashSet<Pubkey> = {
            let mut cumilative_stake = 0;
            let epoch = root_bank.get_leader_schedule_epoch(slot);
            root_bank
                .epoch_staked_nodes(epoch)
                .unwrap()
                .iter()
                .filter(|(pubkey, _)| **pubkey != self_pubkey)
                .sorted_by_key(|(pubkey, stake)| (**stake, **pubkey))
                .take_while(|(_, stake)| {
                    cumilative_stake += *stake;
                    cumilative_stake <= self.config.stake_partition
                })
                .map(|(pubkey, _)| *pubkey)
                .collect()
        };

        // Broadcast data
        let cluster_nodes =
            self.cluster_nodes_cache
                .get(slot, &root_bank, &working_bank, cluster_info);
        let socket_addr_space = cluster_info.socket_addr_space();
        let packets: Vec<_> = shreds
            .iter()
            .filter_map(|shred| {
                let addr = cluster_nodes
                    .get_broadcast_addrs(shred, &root_bank, DATA_PLANE_FANOUT, socket_addr_space)
                    .first()
                    .copied()?;
                let node = nodes.iter().find(|node| node.tvu == addr)?;
                if !socket_addr_space.check(&node.tvu) {
                    return None;
                }
                if self
                    .original_last_data_shreds
                    .lock()
                    .unwrap()
                    .remove(&shred.signature())
                {
                    if cluster_partition.contains(&node.id) {
                        info!(
                            "skipping node {} for original shred index {}, slot {}",
                            node.id,
                            shred.index(),
                            shred.slot()
                        );
                        return None;
                    }
                } else if self
                    .partition_last_data_shreds
                    .lock()
                    .unwrap()
                    .remove(&shred.signature())
                {
                    // If the shred is part of the partition, broadcast it directly to the
                    // partition node. This is to account for cases when the partition stake
                    // is small such as in `test_duplicate_shreds_broadcast_leader()`, then
                    // the partition node is never selected by get_broadcast_peer()
                    return Some(
                        cluster_partition
                            .iter()
                            .filter_map(|pubkey| {
                                let tvu = cluster_info
                                    .lookup_contact_info(pubkey, |contact_info| contact_info.tvu)?;
                                Some((&shred.payload, tvu))
                            })
                            .collect(),
                    );
                }

                Some(vec![(&shred.payload, node.tvu)])
            })
            .flatten()
            .collect();

        if let Err(SendPktsError::IoError(ioerr, _)) = batch_send(sock, &packets) {
            return Err(Error::Io(ioerr));
        }
        Ok(())
    }
