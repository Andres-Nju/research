    fn test_quic_timeout() {
        solana_logger::setup();
        let (t, exit, receiver, server_address) = setup_quic_server();

        let runtime = rt();
        let _rt_guard = runtime.enter();
        let conn1 = make_client_endpoint(&runtime, &server_address);
        let total = 30;
        let handle = runtime.spawn(async move {
            for i in 0..total {
                let mut s1 = conn1.connection.open_uni().await.unwrap();
                s1.write_all(&[0u8]).await.unwrap();
                s1.finish().await.unwrap();
                info!("done {}", i);
                std::thread::sleep(Duration::from_millis(1000));
            }
        });
        let mut received = 0;
        loop {
            if let Ok(_x) = receiver.recv_timeout(Duration::from_millis(500)) {
                received += 1;
                info!("got {}", received);
            }
            if received >= total {
                break;
            }
        }
        runtime.block_on(handle).unwrap();
        exit.store(true, Ordering::Relaxed);
        t.join().unwrap();
    }
