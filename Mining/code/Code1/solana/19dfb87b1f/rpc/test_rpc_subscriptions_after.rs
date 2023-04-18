fn test_rpc_subscriptions() {
    solana_logger::setup();

    let TestValidator {
        server,
        leader_data,
        alice,
        ledger_path,
        genesis_hash,
        ..
    } = TestValidator::run();

    // Create transaction signatures to subscribe to
    let transactions_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let transactions: Vec<Transaction> = (0..500)
        .map(|_| system_transaction::transfer(&alice, &Pubkey::new_rand(), 1, genesis_hash))
        .collect();
    let mut signature_set: HashSet<String> = transactions
        .iter()
        .map(|tx| tx.signatures[0].to_string())
        .collect();

    // Create the pub sub runtime
    let mut rt = Runtime::new().unwrap();
    let rpc_pubsub_url = format!("ws://{}/", leader_data.rpc_pubsub);

    let (status_sender, status_receiver) = channel::<(String, Response<transaction::Result<()>>)>();
    let status_sender = Arc::new(Mutex::new(status_sender));
    let (sent_sender, sent_receiver) = channel::<()>();
    let sent_sender = Arc::new(Mutex::new(sent_sender));

    // Subscribe to all signatures
    rt.spawn({
        let connect = ws::try_connect::<PubsubClient>(&rpc_pubsub_url).unwrap();
        let signature_set = signature_set.clone();
        connect
            .and_then(move |client| {
                for sig in signature_set {
                    let status_sender = status_sender.clone();
                    let sent_sender = sent_sender.clone();
                    tokio::spawn(
                        client
                            .signature_subscribe(sig.clone(), None)
                            .and_then(move |sig_stream| {
                                sent_sender.lock().unwrap().send(()).unwrap();
                                sig_stream.for_each(move |result| {
                                    status_sender
                                        .lock()
                                        .unwrap()
                                        .send((sig.clone(), result))
                                        .unwrap();
                                    future::ok(())
                                })
                            })
                            .map_err(|err| {
                                eprintln!("sig sub err: {:#?}", err);
                            }),
                    );
                }
                future::ok(())
            })
            .map_err(|_| ())
    });

    // Wait for signature subscriptions
    let deadline = Instant::now() + Duration::from_secs(2);
    (0..transactions.len()).for_each(|_| {
        sent_receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
    });

    let rpc_client = RpcClient::new_socket(leader_data.rpc);
    let mut transaction_count = rpc_client
        .get_transaction_count_with_commitment(CommitmentConfig::recent())
        .unwrap();

    // Send all transactions to tpu socket for processing
    transactions.iter().for_each(|tx| {
        transactions_socket
            .send_to(&bincode::serialize(&tx).unwrap(), leader_data.tpu)
            .unwrap();
    });
    let now = Instant::now();
    let expected_transaction_count = transaction_count + transactions.len() as u64;
    while transaction_count < expected_transaction_count && now.elapsed() < Duration::from_secs(5) {
        transaction_count = rpc_client
            .get_transaction_count_with_commitment(CommitmentConfig::recent())
            .unwrap();
        sleep(Duration::from_millis(200));
    }

    // Wait for all signature subscriptions
    let deadline = Instant::now() + Duration::from_secs(5);
    while !signature_set.is_empty() {
        let timeout = deadline.saturating_duration_since(Instant::now());
        match status_receiver.recv_timeout(timeout) {
            Ok((sig, result)) => {
                assert!(result.value.is_ok());
                assert!(signature_set.remove(&sig));
            }
            Err(_err) => {
                eprintln!(
                    "recv_timeout, {}/{} signatures remaining",
                    signature_set.len(),
                    transactions.len()
                );
                assert!(false)
            }
        }
    }

    rt.shutdown_now().wait().unwrap();
    server.close().unwrap();
    remove_dir_all(ledger_path).unwrap();
}
