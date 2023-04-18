    fn process_entries<T: KeypairUtil>(
        mut entries: Vec<Entry>,
        bank: &Arc<Bank>,
        cluster_info: &Arc<RwLock<ClusterInfo>>,
        voting_keypair: &Option<Arc<T>>,
        forward_entry_sender: &EntrySender,
        current_blob_index: &mut u64,
        last_entry_id: &mut Hash,
        subscriptions: &Arc<RpcSubscriptions>,
    ) -> Result<()> {
        // Coalesce all the available entries into a single vote
        submit(
            influxdb::Point::new("replicate-stage")
                .add_field("count", influxdb::Value::Integer(entries.len() as i64))
                .to_owned(),
        );

        let mut res = Ok(());
        let mut num_entries_to_write = entries.len();
        let now = Instant::now();

        if !entries.as_slice().verify(last_entry_id) {
            inc_new_counter_info!("replicate_stage-verify-fail", entries.len());
            return Err(Error::BlobError(BlobError::VerificationFailed));
        }
        inc_new_counter_info!(
            "replicate_stage-verify-duration",
            duration_as_ms(&now.elapsed()) as usize
        );

        let num_ticks = bank.tick_height();

        let mut num_ticks_to_next_vote =
            leader_schedule_utils::num_ticks_left_in_slot(bank, num_ticks);

        for (i, entry) in entries.iter().enumerate() {
            inc_new_counter_info!("replicate-stage_bank-tick", bank.tick_height() as usize);
            if entry.is_tick() {
                if num_ticks_to_next_vote == 0 {
                    num_ticks_to_next_vote = bank.ticks_per_slot();
                }
                num_ticks_to_next_vote -= 1;
            }
            inc_new_counter_info!(
                "replicate-stage_tick-to-vote",
                num_ticks_to_next_vote as usize
            );
            // If it's the last entry in the vector, i will be vec len - 1.
            // If we don't process the entry now, the for loop will exit and the entry
            // will be dropped.
            if 0 == num_ticks_to_next_vote || (i + 1) == entries.len() {
                res = blocktree_processor::process_entries(bank, &entries[0..=i]);

                if res.is_err() {
                    // TODO: This will return early from the first entry that has an erroneous
                    // transaction, instead of processing the rest of the entries in the vector
                    // of received entries. This is in line with previous behavior when
                    // bank.process_entries() was used to process the entries, but doesn't solve the
                    // issue that the bank state was still changed, leading to inconsistencies with the
                    // leader as the leader currently should not be publishing erroneous transactions
                    inc_new_counter_info!("replicate-stage_failed_process_entries", i);
                    break;
                }

                if 0 == num_ticks_to_next_vote {
                    subscriptions.notify_subscribers(&bank);
                    if let Some(voting_keypair) = voting_keypair {
                        let keypair = voting_keypair.as_ref();
                        let vote = VoteTransaction::new_vote(keypair, bank.id(), bank.last_id(), 0);
                        cluster_info.write().unwrap().push_vote(vote);
                    }
                }
                num_entries_to_write = i + 1;
                break;
            }
        }

        // If leader rotation happened, only write the entries up to leader rotation.
        entries.truncate(num_entries_to_write);
        *last_entry_id = entries
            .last()
            .expect("Entries cannot be empty at this point")
            .id;

        inc_new_counter_info!(
            "replicate-transactions",
            entries.iter().map(|x| x.transactions.len()).sum()
        );

        let entries_len = entries.len() as u64;
        // TODO: In line with previous behavior, this will write all the entries even if
        // an error occurred processing one of the entries (causing the rest of the entries to
        // not be processed).
        if entries_len != 0 {
            forward_entry_sender.send(entries)?;
        }

        *current_blob_index += entries_len;
        res?;
        inc_new_counter_info!(
            "replicate_stage-duration",
            duration_as_ms(&now.elapsed()) as usize
        );
        Ok(())
    }
