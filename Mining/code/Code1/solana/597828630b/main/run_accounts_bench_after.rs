fn run_accounts_bench(
    entrypoint_addr: SocketAddr,
    faucet_addr: SocketAddr,
    payer_keypairs: &[&Keypair],
    iterations: usize,
    maybe_space: Option<u64>,
    batch_size: usize,
    close_nth_batch: u64,
    maybe_lamports: Option<u64>,
    num_instructions: usize,
    mint: Option<Pubkey>,
    reclaim_accounts: bool,
) {
    assert!(num_instructions > 0);
    let client =
        RpcClient::new_socket_with_commitment(entrypoint_addr, CommitmentConfig::confirmed());

    info!("Targeting {}", entrypoint_addr);

    let mut latest_blockhash = Instant::now();
    let mut last_log = Instant::now();
    let mut count = 0;
    let mut blockhash = client.get_latest_blockhash().expect("blockhash");
    let mut tx_sent_count = 0;
    let mut total_accounts_created = 0;
    let mut total_accounts_closed = 0;
    let mut balances: Vec<_> = payer_keypairs
        .iter()
        .map(|keypair| client.get_balance(&keypair.pubkey()).unwrap_or(0))
        .collect();
    let mut last_balance = Instant::now();

    let default_max_lamports = 1000;
    let min_balance = maybe_lamports.unwrap_or_else(|| {
        let space = maybe_space.unwrap_or(default_max_lamports);
        client
            .get_minimum_balance_for_rent_exemption(space as usize)
            .expect("min balance")
    });

    let base_keypair = Keypair::new();
    let seed_tracker = SeedTracker {
        max_created: Arc::new(AtomicU64::default()),
        max_closed: Arc::new(AtomicU64::default()),
    };

    info!("Starting balance(s): {:?}", balances);

    let executor = TransactionExecutor::new(entrypoint_addr);

    // Create and close messages both require 2 signatures, fake a 2 signature message to calculate fees
    let mut message = Message::new(
        &[
            Instruction::new_with_bytes(
                Pubkey::new_unique(),
                &[],
                vec![AccountMeta::new(Pubkey::new_unique(), true)],
            ),
            Instruction::new_with_bytes(
                Pubkey::new_unique(),
                &[],
                vec![AccountMeta::new(Pubkey::new_unique(), true)],
            ),
        ],
        None,
    );

    loop {
        if latest_blockhash.elapsed().as_millis() > 10_000 {
            blockhash = client.get_latest_blockhash().expect("blockhash");
            latest_blockhash = Instant::now();
        }

        message.recent_blockhash = blockhash;
        let fee = client
            .get_fee_for_message(&message)
            .expect("get_fee_for_message");
        let lamports = min_balance + fee;

        for (i, balance) in balances.iter_mut().enumerate() {
            if *balance < lamports || last_balance.elapsed().as_millis() > 2000 {
                if let Ok(b) = client.get_balance(&payer_keypairs[i].pubkey()) {
                    *balance = b;
                }
                last_balance = Instant::now();
                if *balance < lamports * 2 {
                    info!(
                        "Balance {} is less than needed: {}, doing airdrop...",
                        balance, lamports
                    );
                    if !airdrop_lamports(
                        &client,
                        &faucet_addr,
                        payer_keypairs[i],
                        lamports * 100_000,
                    ) {
                        warn!("failed airdrop, exiting");
                        return;
                    }
                }
            }
        }

        // Create accounts
        let sigs_len = executor.num_outstanding();
        if sigs_len < batch_size {
            let num_to_create = batch_size - sigs_len;
            if num_to_create >= payer_keypairs.len() {
                info!("creating {} new", num_to_create);
                let chunk_size = num_to_create / payer_keypairs.len();
                if chunk_size > 0 {
                    for (i, keypair) in payer_keypairs.iter().enumerate() {
                        let txs: Vec<_> = (0..chunk_size)
                            .into_par_iter()
                            .map(|_| {
                                let message = make_create_message(
                                    keypair,
                                    &base_keypair,
                                    seed_tracker.max_created.clone(),
                                    num_instructions,
                                    min_balance,
                                    maybe_space,
                                    mint,
                                );
                                let signers: Vec<&Keypair> = vec![keypair, &base_keypair];
                                Transaction::new(&signers, message, blockhash)
                            })
                            .collect();
                        balances[i] = balances[i].saturating_sub(lamports * txs.len() as u64);
                        info!("txs: {}", txs.len());
                        let new_ids = executor.push_transactions(txs);
                        info!("ids: {}", new_ids.len());
                        tx_sent_count += new_ids.len();
                        total_accounts_created += num_instructions * new_ids.len();
                    }
                }
            }

            if close_nth_batch > 0 {
                let num_batches_to_close =
                    total_accounts_created as u64 / (close_nth_batch * batch_size as u64);
                let expected_closed = num_batches_to_close * batch_size as u64;
                let max_closed_seed = seed_tracker.max_closed.load(Ordering::Relaxed);
                // Close every account we've created with seed between max_closed_seed..expected_closed
                if max_closed_seed < expected_closed {
                    let txs: Vec<_> = (0..expected_closed - max_closed_seed)
                        .into_par_iter()
                        .map(|_| {
                            let message = make_close_message(
                                payer_keypairs[0],
                                &base_keypair,
                                seed_tracker.max_created.clone(),
                                seed_tracker.max_closed.clone(),
                                1,
                                min_balance,
                                mint.is_some(),
                            );
                            let signers: Vec<&Keypair> = vec![payer_keypairs[0], &base_keypair];
                            Transaction::new(&signers, message, blockhash)
                        })
                        .collect();
                    balances[0] = balances[0].saturating_sub(fee * txs.len() as u64);
                    info!("close txs: {}", txs.len());
                    let new_ids = executor.push_transactions(txs);
                    info!("close ids: {}", new_ids.len());
                    tx_sent_count += new_ids.len();
                    total_accounts_closed += new_ids.len() as u64;
                }
            }
        } else {
            let _ = executor.drain_cleared();
        }

        count += 1;
        if last_log.elapsed().as_millis() > 3000 || (count >= iterations && iterations != 0) {
            info!(
                "total_accounts_created: {} total_accounts_closed: {} tx_sent_count: {} loop_count: {} balance(s): {:?}",
                total_accounts_created, total_accounts_closed, tx_sent_count, count, balances
            );
            last_log = Instant::now();
        }
        if iterations != 0 && count >= iterations {
            break;
        }
        if executor.num_outstanding() >= batch_size {
            sleep(Duration::from_millis(500));
        }
    }
    executor.close();

    if reclaim_accounts {
        let executor = TransactionExecutor::new(entrypoint_addr);
        loop {
            let max_closed_seed = seed_tracker.max_closed.load(Ordering::Relaxed);
            let max_created_seed = seed_tracker.max_created.load(Ordering::Relaxed);

            if latest_blockhash.elapsed().as_millis() > 10_000 {
                blockhash = client.get_latest_blockhash().expect("blockhash");
                latest_blockhash = Instant::now();
            }
            message.recent_blockhash = blockhash;
            let fee = client
                .get_fee_for_message(&message)
                .expect("get_fee_for_message");

            let sigs_len = executor.num_outstanding();
            if sigs_len < batch_size && max_closed_seed < max_created_seed {
                let num_to_close = min(
                    batch_size - sigs_len,
                    (max_created_seed - max_closed_seed) as usize,
                );
                if num_to_close >= payer_keypairs.len() {
                    info!("closing {} accounts", num_to_close);
                    let chunk_size = num_to_close / payer_keypairs.len();
                    info!("{:?} chunk_size", chunk_size);
                    if chunk_size > 0 {
                        for (i, keypair) in payer_keypairs.iter().enumerate() {
                            let txs: Vec<_> = (0..chunk_size)
                                .into_par_iter()
                                .filter_map(|_| {
                                    let message = make_close_message(
                                        keypair,
                                        &base_keypair,
                                        seed_tracker.max_created.clone(),
                                        seed_tracker.max_closed.clone(),
                                        num_instructions,
                                        min_balance,
                                        mint.is_some(),
                                    );
                                    if message.instructions.is_empty() {
                                        return None;
                                    }
                                    let signers: Vec<&Keypair> = vec![keypair, &base_keypair];
                                    Some(Transaction::new(&signers, message, blockhash))
                                })
                                .collect();
                            balances[i] = balances[i].saturating_sub(fee * txs.len() as u64);
                            info!("close txs: {}", txs.len());
                            let new_ids = executor.push_transactions(txs);
                            info!("close ids: {}", new_ids.len());
                            tx_sent_count += new_ids.len();
                            total_accounts_closed += (num_instructions * new_ids.len()) as u64;
                        }
                    }
                }
            } else {
                let _ = executor.drain_cleared();
            }
            count += 1;
            if last_log.elapsed().as_millis() > 3000 || max_closed_seed >= max_created_seed {
                info!(
                    "total_accounts_closed: {} tx_sent_count: {} loop_count: {} balance(s): {:?}",
                    total_accounts_closed, tx_sent_count, count, balances
                );
                last_log = Instant::now();
            }

            if max_closed_seed >= max_created_seed {
                break;
            }
            if executor.num_outstanding() >= batch_size {
                sleep(Duration::from_millis(500));
            }
        }
        executor.close();
    }
}
