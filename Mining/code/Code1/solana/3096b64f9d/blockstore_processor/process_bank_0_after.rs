fn process_bank_0(
    bank0: &Arc<Bank>,
    blockstore: &Blockstore,
    opts: &ProcessOptions,
    recyclers: &VerifyRecyclers,
    cache_block_meta_sender: Option<&CacheBlockMetaSender>,
) {
    assert_eq!(bank0.slot(), 0);
    let mut progress = ConfirmationProgress::new(bank0.last_blockhash());
    confirm_full_slot(
        blockstore,
        bank0,
        opts,
        recyclers,
        &mut progress,
        None,
        None,
        &mut ExecuteTimings::default(),
    )
    .expect("Failed to process bank 0 from ledger. Did you forget to provide a snapshot?");
    bank0.freeze();
    if blockstore.is_primary_access() {
        blockstore.insert_bank_hash(bank0.slot(), bank0.hash(), false);
    }
    cache_block_meta(bank0, cache_block_meta_sender);
}
