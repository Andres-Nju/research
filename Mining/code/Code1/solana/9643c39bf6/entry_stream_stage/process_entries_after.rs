    fn process_entries(
        ledger_entry_receiver: &EntryReceiver,
        entry_stream_sender: &EntrySender,
        entry_stream: &mut EntryStream,
    ) -> Result<()> {
        let timeout = Duration::new(1, 0);
        let entries = ledger_entry_receiver.recv_timeout(timeout)?;
        let leader_scheduler = entry_stream.leader_scheduler.read().unwrap();

        for entry in &entries {
            let slot = leader_scheduler.tick_height_to_slot(entry.tick_height);
            let leader_id = leader_scheduler
                .get_leader_for_slot(slot)
                .map(|leader| leader.to_string())
                .unwrap_or_else(|| "None".to_string());

            if entry.is_tick() && entry_stream.queued_block.is_some() {
                let queued_block = entry_stream.queued_block.as_ref();
                let block_slot = queued_block.unwrap().slot;
                let block_tick_height = queued_block.unwrap().tick_height;
                let block_id = queued_block.unwrap().id;
                entry_stream
                    .emit_block_event(block_slot, &leader_id, block_tick_height, block_id)
                    .unwrap_or_else(|e| {
                        error!("Entry Stream error: {:?}, {:?}", e, entry_stream.output);
                    });
                entry_stream.queued_block = None;
            }
            entry_stream
                .emit_entry_event(slot, &leader_id, &entry)
                .unwrap_or_else(|e| {
                    error!("Entry Stream error: {:?}, {:?}", e, entry_stream.output);
                });
            if 0 == leader_scheduler.num_ticks_left_in_slot(entry.tick_height) {
                entry_stream.queued_block = Some(EntryStreamBlock {
                    slot,
                    tick_height: entry.tick_height,
                    id: entry.id,
                });
            }
        }

        entry_stream_sender.send(entries)?;
        Ok(())
    }
