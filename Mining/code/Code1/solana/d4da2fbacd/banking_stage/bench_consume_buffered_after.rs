fn bench_consume_buffered(bencher: &mut Bencher) {
    let (genesis_block, _mint_keypair) = create_genesis_block(100_000);
    let bank = Arc::new(Bank::new(&genesis_block));
    let ledger_path = get_tmp_ledger_path!();
    {
        let blocktree = Arc::new(
            Blocktree::open(&ledger_path).expect("Expected to be able to open database ledger"),
        );
        let (exit, poh_recorder, poh_service, _signal_receiver) =
            create_test_recorder(&bank, &blocktree);

        let tx = test_tx();
        let len = 4096;
        let chunk_size = 1024;
        let batches = to_packets_chunked(&vec![tx; len], chunk_size);
        let mut packets = vec![];
        for batch in batches {
            let batch_len = batch.packets.len();
            packets.push((Rc::new(batch), vec![0usize; batch_len]));
        }
        // This tests the performance of buffering packets.
        // If the packet buffers are copied, performance will be poor.
        bencher.iter(move || {
            let _ignored =
                BankingStage::consume_buffered_packets(&poh_recorder, packets.as_slice());
        });

        exit.store(true, Ordering::Relaxed);
        poh_service.join().unwrap();
    }
    let _unused = Blocktree::destroy(&ledger_path);
}
