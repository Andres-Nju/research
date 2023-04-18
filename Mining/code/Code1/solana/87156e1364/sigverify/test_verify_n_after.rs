    fn test_verify_n(n: usize, modify_data: bool) {
        let tx = test_tx();
        let mut packet = make_packet_from_transaction(tx);

        // jumble some data to test failure
        if modify_data {
            packet.data[20] = packet.data[20].wrapping_add(10);
        }

        // generate packet vector
        let mut packets = Packets::default();
        packets.packets = Vec::new();
        for _ in 0..n {
            packets.packets.push(packet.clone());
        }
        let shared_packets = SharedPackets::new(RwLock::new(packets));
        let batches = vec![shared_packets.clone(), shared_packets.clone()];

        // verify packets
        let ans = sigverify::ed25519_verify(&batches);

        // check result
        let ref_ans = if modify_data { 0u8 } else { 1u8 };
        assert_eq!(ans, vec![vec![ref_ans; n], vec![ref_ans; n]]);
    }
