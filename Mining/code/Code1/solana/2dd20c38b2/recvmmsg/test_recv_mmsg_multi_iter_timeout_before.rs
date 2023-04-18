    pub fn test_recv_mmsg_multi_iter_timeout() {
        let reader = UdpSocket::bind("127.0.0.1:0").expect("bind");
        let addr = reader.local_addr().unwrap();
        reader.set_read_timeout(Some(Duration::new(5, 0))).unwrap();
        reader.set_nonblocking(false).unwrap();
        let sender = UdpSocket::bind("127.0.0.1:0").expect("bind");
        let saddr = sender.local_addr().unwrap();
        let sent = NUM_RCVMMSGS;
        for _ in 0..sent {
            let data = [0; PACKET_DATA_SIZE];
            sender.send_to(&data[..], &addr).unwrap();
        }

        let start = Instant::now();
        let mut packets = vec![Packet::default(); NUM_RCVMMSGS * 2];
        let recv = recv_mmsg(&reader, &mut packets[..]).unwrap();
        assert_eq!(NUM_RCVMMSGS, recv);
        for i in 0..recv {
            assert_eq!(packets[i].meta.size, PACKET_DATA_SIZE);
            assert_eq!(packets[i].meta.addr(), saddr);
        }

        let _recv = recv_mmsg(&reader, &mut packets[..]);
        assert!(start.elapsed().as_secs() < 5);
    }
