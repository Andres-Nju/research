    fn streamer_send_test() {
        let read = UdpSocket::bind("127.0.0.1:0").expect("bind");
        read.set_read_timeout(Some(Duration::new(1, 0))).unwrap();

        let addr = read.local_addr().unwrap();
        let send = UdpSocket::bind("127.0.0.1:0").expect("bind");
        let exit = Arc::new(AtomicBool::new(false));
        let (s_reader, r_reader) = unbounded();
        let stats = Arc::new(StreamerReceiveStats::new("test"));
        let t_receiver = receiver(
            Arc::new(read),
            exit.clone(),
            s_reader,
            Recycler::default(),
            stats.clone(),
            1,
            true,
            None,
        );
        const NUM_PACKETS: usize = 5;
        let t_responder = {
            let (s_responder, r_responder) = unbounded();
            let t_responder = responder(
                "streamer_send_test",
                Arc::new(send),
                r_responder,
                SocketAddrSpace::Unspecified,
                None,
            );
            let mut packet_batch = PacketBatch::default();
            for i in 0..NUM_PACKETS {
                let mut p = Packet::default();
                {
                    p.data[0] = i as u8;
                    p.meta.size = PACKET_DATA_SIZE;
                    p.meta.set_addr(&addr);
                }
                packet_batch.packets.push(p);
            }
            s_responder.send(packet_batch).expect("send");
            t_responder
        };

        let mut packets_remaining = NUM_PACKETS;
        get_packet_batches(r_reader, &mut packets_remaining);
        assert_eq!(packets_remaining, 0);
        exit.store(true, Ordering::Relaxed);
        assert!(stats.packet_batches_count.load(Ordering::Relaxed) >= 1);
        assert_eq!(stats.packets_count.load(Ordering::Relaxed), NUM_PACKETS);
        assert_eq!(stats.full_packet_batches_count.load(Ordering::Relaxed), 0);
        t_receiver.join().expect("join");
        t_responder.join().expect("join");
    }
