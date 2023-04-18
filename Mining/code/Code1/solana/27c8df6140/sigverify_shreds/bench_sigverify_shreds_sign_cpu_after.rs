fn bench_sigverify_shreds_sign_cpu(bencher: &mut Bencher) {
    let mut packets = Packets::default();
    let slot = 0xdeadc0de;
    packets.packets.resize(NUM_PACKETS, Packet::default());
    for p in packets.packets.iter_mut() {
        let shred = Shred::new_from_data(
            slot,
            0xc0de,
            0xdead,
            Some(&[5; SIZE_OF_DATA_SHRED_PAYLOAD]),
            true,
            true,
            1,
            2,
            0,
        );
        shred.copy_to_packet(p);
    }
    let mut batch = vec![packets; NUM_BATCHES];
    let keypair = Keypair::new();
    bencher.iter(|| {
        sign_shreds_cpu(&keypair, &mut batch);
    })
}
