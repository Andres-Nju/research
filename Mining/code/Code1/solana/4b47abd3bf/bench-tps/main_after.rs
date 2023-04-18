fn main() {
    logger::setup();
    metrics::set_panic_hook("bench-tps");
    let mut threads = 4usize;
    let mut num_nodes = 1usize;
    let mut time_sec = 90;
    let mut sustained = false;
    let mut tx_count = 500_000;

    let matches = App::new("solana-bench-tps")
        .version(crate_version!())
        .arg(
            Arg::with_name("network")
                .short("n")
                .long("network")
                .value_name("HOST:PORT")
                .takes_value(true)
                .required(true)
                .help("rendezvous with the network at this gossip entry point"),
        )
        .arg(
            Arg::with_name("keypair")
                .short("k")
                .long("keypair")
                .value_name("PATH")
                .takes_value(true)
                .default_value("~/.config/solana/id.json")
                .help("/path/to/id.json"),
        )
        .arg(
            Arg::with_name("num-nodes")
                .short("N")
                .long("num-nodes")
                .value_name("NUM")
                .takes_value(true)
                .help("wait for NUM nodes to converge"),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .value_name("NUM")
                .takes_value(true)
                .help("number of threads"),
        )
        .arg(
            Arg::with_name("seconds")
                .short("s")
                .long("seconds")
                .value_name("NUM")
                .takes_value(true)
                .help("send transactions for this many seconds"),
        )
        .arg(
            Arg::with_name("converge-only")
                .short("c")
                .long("converge-only")
                .help("exit immediately after converging"),
        )
        .arg(
            Arg::with_name("sustained")
                .long("sustained")
                .help("Use sustained performance mode vs. peak mode. This overlaps the tx generation with transfers."),
        )
        .arg(
            Arg::with_name("tx_count")
                .long("tx_count")
                .value_name("NUMBER")
                .takes_value(true)
                .help("number of transactions to send in a single batch")
        )
        .get_matches();

    let network = matches
        .value_of("network")
        .unwrap()
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("failed to parse network: {}", e);
            exit(1)
        });

    let id = read_keypair(matches.value_of("keypair").unwrap()).expect("client keypair");

    if let Some(t) = matches.value_of("threads") {
        threads = t.to_string().parse().expect("integer");
    }

    if let Some(n) = matches.value_of("num-nodes") {
        num_nodes = n.to_string().parse().expect("integer");
    }

    if let Some(s) = matches.value_of("seconds") {
        time_sec = s.to_string().parse().expect("integer");
    }

    if let Some(s) = matches.value_of("tx_count") {
        tx_count = s.to_string().parse().expect("integer");
    }

    if matches.is_present("sustained") {
        sustained = true;
    }

    let leader = poll_gossip_for_leader(network, None).expect("unable to find leader on network");

    let exit_signal = Arc::new(AtomicBool::new(false));
    let mut c_threads = vec![];
    let (validators, leader) = converge(&leader, &exit_signal, num_nodes, &mut c_threads);

    println!(" Node address         | Node identifier");
    println!("----------------------+------------------");
    for node in &validators {
        println!(" {:20} | {}", node.contact_info.tpu.to_string(), node.id);
    }
    println!("Nodes: {}", validators.len());

    if validators.len() < num_nodes {
        println!(
            "Error: Insufficient nodes discovered.  Expecting {} or more",
            num_nodes
        );
        exit(1);
    }
    if leader.is_none() {
        println!("no leader");
        exit(1);
    }

    if matches.is_present("converge-only") {
        return;
    }

    let leader = leader.unwrap();

    println!("leader is at {} {}", leader.contact_info.rpu, leader.id);
    let mut client = mk_client(&leader);
    let mut barrier_client = mk_client(&leader);

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&id.public_key_bytes()[..32]);
    let mut rnd = GenKeys::new(seed);

    println!("Creating {} keypairs...", tx_count / 2);
    let keypairs = rnd.gen_n_keypairs(tx_count / 2);
    let barrier_id = rnd.gen_n_keypairs(1).pop().unwrap();

    println!("Get tokens...");
    let num_tokens_per_account = 20;

    // Sample the first keypair, see if it has tokens, if so then resume
    // to avoid token loss
    let keypair0_balance = client.poll_get_balance(&keypairs[0].pubkey()).unwrap_or(0);

    if num_tokens_per_account > keypair0_balance {
        airdrop_tokens(
            &mut client,
            &leader,
            &id,
            (num_tokens_per_account - keypair0_balance) * tx_count,
        );
    }
    airdrop_tokens(&mut barrier_client, &leader, &barrier_id, 1);

    println!("Get last ID...");
    let mut last_id = client.get_last_id();
    println!("Got last ID {:?}", last_id);

    let first_tx_count = client.transaction_count();
    println!("Initial transaction count {}", first_tx_count);

    // Setup a thread per validator to sample every period
    // collect the max transaction rate and total tx count seen
    let maxes = Arc::new(RwLock::new(Vec::new()));
    let sample_period = 1; // in seconds
    println!("Sampling TPS every {} second...", sample_period);
    let v_threads: Vec<_> = validators
        .into_iter()
        .map(|v| {
            let exit_signal = exit_signal.clone();
            let maxes = maxes.clone();
            Builder::new()
                .name("solana-client-sample".to_string())
                .spawn(move || {
                    sample_tx_count(&exit_signal, &maxes, first_tx_count, &v, sample_period);
                })
                .unwrap()
        })
        .collect();

    let shared_txs: Arc<RwLock<VecDeque<Vec<Transaction>>>> =
        Arc::new(RwLock::new(VecDeque::new()));

    let shared_tx_active_thread_count = Arc::new(AtomicIsize::new(0));
    let total_tx_sent_count = Arc::new(AtomicUsize::new(0));

    let s_threads: Vec<_> = (0..threads)
        .map(|_| {
            let exit_signal = exit_signal.clone();
            let shared_txs = shared_txs.clone();
            let leader = leader.clone();
            let shared_tx_active_thread_count = shared_tx_active_thread_count.clone();
            let total_tx_sent_count = total_tx_sent_count.clone();
            Builder::new()
                .name("solana-client-sender".to_string())
                .spawn(move || {
                    do_tx_transfers(
                        &exit_signal,
                        &shared_txs,
                        &leader,
                        &shared_tx_active_thread_count,
                        &total_tx_sent_count,
                    );
                })
                .unwrap()
        })
        .collect();

    // generate and send transactions for the specified duration
    let time = Duration::new(time_sec, 0);
    let now = Instant::now();
    let mut reclaim_tokens_back_to_source_account = false;
    let mut i = keypair0_balance;
    while now.elapsed() < time {
        let balance = client.poll_get_balance(&id.pubkey()).unwrap_or(-1);
        metrics_submit_token_balance(balance);

        // ping-pong between source and destination accounts for each loop iteration
        // this seems to be faster than trying to determine the balance of individual
        // accounts
        generate_txs(
            &shared_txs,
            &id,
            &keypairs,
            &last_id,
            threads,
            reclaim_tokens_back_to_source_account,
        );
        // In sustained mode overlap the transfers with generation
        // this has higher average performance but lower peak performance
        // in tested environments.
        if !sustained {
            while shared_tx_active_thread_count.load(Ordering::Relaxed) > 0 {
                sleep(Duration::from_millis(100));
            }
        }
        // It's not feasible (would take too much time) to confirm each of the `tx_count / 2`
        // transactions sent by `generate_txs()` so instead send and confirm a single transaction
        // to validate the network is still functional.
        send_barrier_transaction(&mut barrier_client, &mut last_id, &barrier_id);

        i += 1;
        if should_switch_directions(num_tokens_per_account, i) {
            reclaim_tokens_back_to_source_account = !reclaim_tokens_back_to_source_account;
        }
    }

    // Stop the sampling threads so it will collect the stats
    exit_signal.store(true, Ordering::Relaxed);

    println!("Waiting for validator threads...");
    for t in v_threads {
        if let Err(err) = t.join() {
            println!("  join() failed with: {:?}", err);
        }
    }

    // join the tx send threads
    println!("Waiting for transmit threads...");
    for t in s_threads {
        if let Err(err) = t.join() {
            println!("  join() failed with: {:?}", err);
        }
    }

    let balance = client.poll_get_balance(&id.pubkey()).unwrap_or(-1);
    metrics_submit_token_balance(balance);

    compute_and_report_stats(
        &maxes,
        sample_period,
        &now.elapsed(),
        total_tx_sent_count.load(Ordering::Relaxed),
    );

    // join the crdt client threads
    for t in c_threads {
        t.join().unwrap();
    }
}
