    fn replay_blocktree_into_bank(
        bank: &Arc<Bank>,
        blocktree: &Blocktree,
        progress: &mut HashMap<u64, ForkProgress>,
    ) -> (Result<()>, usize) {
        let mut tx_count = 0;
        let bank_progress = &mut progress
            .entry(bank.slot())
            .or_insert_with(|| ForkProgress::new(bank.slot(), bank.last_blockhash()));
        let now = Instant::now();
        let load_result =
            Self::load_blocktree_entries_with_shred_info(bank, blocktree, bank_progress);
        let fetch_entries_elapsed = now.elapsed().as_micros();
        if load_result.is_err() {
            bank_progress.stats.fetch_entries_fail_elapsed += fetch_entries_elapsed as u64;
        } else {
            bank_progress.stats.fetch_entries_elapsed += fetch_entries_elapsed as u64;
        }

        let replay_result = load_result.and_then(|(entries, num_shreds, slot_full)| {
            trace!(
                "Fetch entries for slot {}, {:?} entries, num shreds {}, slot_full: {}",
                bank.slot(),
                entries.len(),
                num_shreds,
                slot_full,
            );
            tx_count += entries.iter().map(|e| e.transactions.len()).sum::<usize>();
            Self::replay_entries_into_bank(bank, bank_progress, entries, num_shreds, slot_full)
        });

        if Self::is_replay_result_fatal(&replay_result) {
            warn!(
                "Fatal replay result in slot: {}, result: {:?}",
                bank.slot(),
                replay_result
            );
            datapoint_warn!("replay-stage-mark_dead_slot", ("slot", bank.slot(), i64),);
            Self::mark_dead_slot(bank.slot(), blocktree, progress);
        }

        (replay_result, tx_count)
    }
