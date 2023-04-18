    fn bank_send_loop(
        exit: Arc<AtomicBool>,
        verified_vote_label_packets_receiver: VerifiedLabelVotePacketsReceiver,
        poh_recorder: Arc<Mutex<PohRecorder>>,
        verified_packets_sender: &CrossbeamSender<Vec<PacketBatch>>,
    ) -> Result<()> {
        let mut verified_vote_packets = VerifiedVotePackets::default();
        let mut time_since_lock = Instant::now();
        let mut bank_vote_sender_state_option: Option<BankVoteSenderState> = None;

        loop {
            if exit.load(Ordering::Relaxed) {
                return Ok(());
            }

            let would_be_leader = poh_recorder
                .lock()
                .unwrap()
                .would_be_leader(3 * slot_hashes::MAX_ENTRIES as u64 * DEFAULT_TICKS_PER_SLOT);

            if let Err(e) = verified_vote_packets.receive_and_process_vote_packets(
                &verified_vote_label_packets_receiver,
                would_be_leader,
            ) {
                match e {
                    Error::CrossbeamRecvTimeout(RecvTimeoutError::Disconnected)
                    | Error::ReadyTimeout => (),
                    _ => {
                        error!("thread {:?} error {:?}", thread::current().name(), e);
                    }
                }
            }

            if time_since_lock.elapsed().as_millis() > BANK_SEND_VOTES_LOOP_SLEEP_MS as u128 {
                // Always set this to avoid taking the poh lock too often
                time_since_lock = Instant::now();
                // We will take this lock at most once every `BANK_SEND_VOTES_LOOP_SLEEP_MS`
                if let Some(current_working_bank) = poh_recorder.lock().unwrap().bank() {
                    Self::check_for_leader_bank_and_send_votes(
                        &mut bank_vote_sender_state_option,
                        current_working_bank,
                        verified_packets_sender,
                        &verified_vote_packets,
                    )?;
                }
            }
        }
    }
