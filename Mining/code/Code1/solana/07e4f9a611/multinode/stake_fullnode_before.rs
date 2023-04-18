fn stake_fullnode(
    node: &Arc<Keypair>,
    stake: u64,
    last_tick: &Hash,
    last_hash: &mut Hash,
    entries: &mut Vec<Entry>,
) -> VotingKeypair {
    // Create and register a vote account for active_keypair
    let voting_keypair = VotingKeypair::new_local(node);
    let vote_account_id = voting_keypair.pubkey();

    let new_vote_account_tx =
        VoteTransaction::new_account(node, vote_account_id, *last_tick, stake, 0);
    let new_vote_account_entry = next_entry_mut(last_hash, 1, vec![new_vote_account_tx]);
    /*
    let vote_tx = VoteTransaction::new_vote(&voting_keypair, 1, *last_tick, 0);
    let vote_entry = next_entry_mut(last_hash, 1, vec![vote_tx]);

    entries.extend(vec![new_vote_account_entry, vote_entry]);
    */
    entries.extend(vec![new_vote_account_entry]);
    voting_keypair
}
