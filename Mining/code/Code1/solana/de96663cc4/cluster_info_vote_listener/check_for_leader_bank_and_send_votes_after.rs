    fn check_for_leader_bank_and_send_votes(
        bank_vote_sender_state_option: &mut Option<BankVoteSenderState>,
        current_working_bank: Arc<Bank>,
        verified_packets_sender: &Sender<Vec<PacketBatch>>,
        verified_vote_packets: &VerifiedVotePackets,
    ) -> Result<()> {
        // We will take this lock at most once every `BANK_SEND_VOTES_LOOP_SLEEP_MS`
        if let Some(bank_vote_sender_state) = bank_vote_sender_state_option {
            if bank_vote_sender_state.bank.slot() != current_working_bank.slot() {
                bank_vote_sender_state.report_metrics();
                *bank_vote_sender_state_option =
                    Some(BankVoteSenderState::new(current_working_bank));
            }
        } else {
            *bank_vote_sender_state_option = Some(BankVoteSenderState::new(current_working_bank));
        }

        let bank_vote_sender_state = bank_vote_sender_state_option.as_mut().unwrap();
        let BankVoteSenderState {
            ref bank,
            ref mut bank_send_votes_stats,
            ref mut previously_sent_to_bank_votes,
        } = bank_vote_sender_state;

        // This logic may run multiple times for the same leader bank,
        // we just have to ensure that the same votes are not sent
        // to the bank multiple times, which is guaranteed by
        // `previously_sent_to_bank_votes`
        let gossip_votes_iterator = ValidatorGossipVotesIterator::new(
            bank.clone(),
            verified_vote_packets,
            previously_sent_to_bank_votes,
        );

        let mut filter_gossip_votes_timing = Measure::start("filter_gossip_votes");

        // Send entire batch at a time so that there is no partial processing of
        // a single validator's votes by two different banks. This might happen
        // if we sent each vote individually, for instance if we created two different
        // leader banks from the same common parent, one leader bank may process
        // only the later votes and ignore the earlier votes.
        for single_validator_votes in gossip_votes_iterator {
            bank_send_votes_stats.num_votes_sent += single_validator_votes.len();
            bank_send_votes_stats.num_batches_sent += 1;
            verified_packets_sender.send(single_validator_votes)?;
        }
        filter_gossip_votes_timing.stop();
        bank_send_votes_stats.total_elapsed += filter_gossip_votes_timing.as_us();

        Ok(())
    }
