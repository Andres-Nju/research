    fn run(
        &mut self,
        cluster_info: &Arc<RwLock<ClusterInfo>>,
        receiver: &Receiver<WorkingBankEntries>,
        sock: &UdpSocket,
        blocktree: &Arc<Blocktree>,
        storage_entry_sender: &EntrySender,
        genesis_blockhash: &Hash,
    ) -> Result<()> {
        let timer = Duration::new(1, 0);
        let (mut bank, entries) = receiver.recv_timeout(timer)?;
        let mut max_tick_height = bank.max_tick_height();

        let now = Instant::now();
        let mut num_entries = entries.len();
        let mut ventries = Vec::new();
        let mut last_tick = entries.last().map(|v| v.1).unwrap_or(0);
        ventries.push(entries);

        assert!(last_tick <= max_tick_height);
        if last_tick != max_tick_height {
            while let Ok((same_bank, entries)) = receiver.try_recv() {
                // If the bank changed, that implies the previous slot was interrupted and we do not have to
                // broadcast its entries.
                if same_bank.slot() != bank.slot() {
                    num_entries = 0;
                    ventries.clear();
                    bank = same_bank.clone();
                    max_tick_height = bank.max_tick_height();
                }
                num_entries += entries.len();
                last_tick = entries.last().map(|v| v.1).unwrap_or(0);
                ventries.push(entries);
                assert!(last_tick <= max_tick_height,);
                if last_tick == max_tick_height {
                    break;
                }
            }
        }

        let bank_epoch = bank.get_stakers_epoch(bank.slot());
        let mut broadcast_table = cluster_info.read().unwrap().sorted_tvu_peers(
            &staking_utils::delegated_stakes_at_epoch(&bank, bank_epoch).unwrap(),
        );
        // Layer 1, leader nodes are limited to the fanout size.
        broadcast_table.truncate(NEIGHBORHOOD_SIZE);

        inc_new_counter_info!("broadcast_service-num_peers", broadcast_table.len() + 1);
        inc_new_counter_info!("broadcast_service-entries_received", num_entries);

        let to_blobs_start = Instant::now();

        let blobs: Vec<_> = ventries
            .into_par_iter()
            .map_with(storage_entry_sender.clone(), |s, p| {
                let entries: Vec<_> = p.into_iter().map(|e| e.0).collect();
                let blobs = entries.to_shared_blobs();
                let _ignored = s.send(entries);
                blobs
            })
            .flatten()
            .collect();

        let blob_index = blocktree
            .meta(bank.slot())
            .expect("Database error")
            .map(|meta| meta.consumed)
            .unwrap_or(0);

        index_blobs_with_genesis(
            &blobs,
            &self.id,
            genesis_blockhash,
            blob_index,
            bank.slot(),
            bank.parent().map_or(0, |parent| parent.slot()),
        );

        let contains_last_tick = last_tick == max_tick_height;

        if contains_last_tick {
            blobs.last().unwrap().write().unwrap().set_is_last_in_slot();
        }

        blocktree.write_shared_blobs(&blobs)?;

        let coding = self.coding_generator.next(&blobs);

        let to_blobs_elapsed = duration_as_ms(&to_blobs_start.elapsed());

        let broadcast_start = Instant::now();

        // Send out data
        ClusterInfo::broadcast(&self.id, contains_last_tick, &broadcast_table, sock, &blobs)?;

        inc_new_counter_info!("streamer-broadcast-sent", blobs.len());

        // send out erasures
        ClusterInfo::broadcast(&self.id, false, &broadcast_table, sock, &coding)?;

        let broadcast_elapsed = duration_as_ms(&broadcast_start.elapsed());

        inc_new_counter_info!(
            "broadcast_service-time_ms",
            duration_as_ms(&now.elapsed()) as usize
        );
        info!(
            "broadcast: {} entries, blob time {} broadcast time {}",
            num_entries, to_blobs_elapsed, broadcast_elapsed
        );

        submit(
            influxdb::Point::new("broadcast-service")
                .add_field(
                    "transmit-index",
                    influxdb::Value::Integer(blob_index as i64),
                )
                .to_owned(),
        );

        Ok(())
    }
