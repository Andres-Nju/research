fn main() {
    env_logger::init();
    let mut threads = 4usize;
    let mut num_nodes = 1usize;
    let mut time_sec = 90;

    let matches = App::new("solana-client-demo")
        .arg(
            Arg::with_name("leader")
                .short("l")
                .long("leader")
                .value_name("PATH")
                .takes_value(true)
                .help("/path/to/leader.json"),
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
            Arg::with_name("num_nodes")
                .short("n")
                .long("nodes")
                .value_name("NUMBER")
                .takes_value(true)
                .help("number of nodes to converge to"),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .value_name("NUMBER")
                .takes_value(true)
                .help("number of threads"),
        )
        .arg(
            Arg::with_name("seconds")
                .short("s")
                .long("sec")
                .value_name("NUMBER")
                .takes_value(true)
                .help("send transactions for this many seconds"),
        )
        .get_matches();

    let leader: NodeInfo;
    if let Some(l) = matches.value_of("leader") {
        leader = read_leader(l).node_info;
    } else {
        let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8000);
        leader = NodeInfo::new_leader(&server_addr);
    };

    let id = read_keypair(matches.value_of("keypair").unwrap()).expect("client keypair");

    if let Some(t) = matches.value_of("threads") {
        threads = t.to_string().parse().expect("integer");
    }

    if let Some(n) = matches.value_of("nodes") {
        num_nodes = n.to_string().parse().expect("integer");
    }

    if let Some(s) = matches.value_of("seconds") {
        time_sec = s.to_string().parse().expect("integer");
    }

    let mut drone_addr = leader.contact_info.tpu;
    drone_addr.set_port(9900);

    let signal = Arc::new(AtomicBool::new(false));
    let mut c_threads = vec![];
    let validators = converge(&leader, &signal.clone(), num_nodes, &mut c_threads);
    assert_eq!(validators.len(), num_nodes);

    let mut client = mk_client(&leader);

    let starting_balance = client.poll_get_balance(&id.pubkey()).unwrap();
    let txs: i64 = 500_000;

    if starting_balance < txs {
        let airdrop_amount = txs - starting_balance;
        println!("Airdropping {:?} tokens", airdrop_amount);
        request_airdrop(&drone_addr, &id, airdrop_amount as u64).unwrap();
        // TODO: return airdrop Result from Drone
        sleep(Duration::from_millis(100));

        let balance = client.poll_get_balance(&id.pubkey()).unwrap();
        println!("Your balance is: {:?}", balance);

        if balance < txs || (starting_balance == balance) {
            println!("TPS airdrop limit reached; wait 60sec to retry");
            exit(1);
        }
    }

    println!("Get last ID...");
    let mut last_id = client.get_last_id();
    println!("Got last ID {:?}", last_id);

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&id.public_key_bytes()[..32]);
    let rnd = GenKeys::new(seed);

    println!("Creating keypairs...");
    let keypairs = rnd.gen_n_keypairs(txs / 2);

    let first_count = client.transaction_count();
    println!("initial count {}", first_count);

    println!("Sampling tps every second...",);

    // Setup a thread per validator to sample every period
    // collect the max transaction rate and total tx count seen
    let maxes = Arc::new(RwLock::new(Vec::new()));
    let sample_period = 1; // in seconds
    let v_threads: Vec<_> = validators
        .into_iter()
        .map(|v| {
            let exit = signal.clone();
            let maxes = maxes.clone();
            Builder::new()
                .name("solana-client-sample".to_string())
                .spawn(move || {
                    sample_tx_count(&exit, &maxes, first_count, &v, sample_period);
                })
                .unwrap()
        })
        .collect();

    let clients: Vec<_> = (0..threads).map(|_| mk_client(&leader)).collect();

    // generate and send transactions for the specified duration
    let time = Duration::new(time_sec / 2, 0);
    let mut now = Instant::now();
    while now.elapsed() < time {
        generate_and_send_txs(
            &mut client,
            &clients,
            &id,
            &keypairs,
            &leader,
            txs,
            &mut last_id,
            threads,
            false,
        );
    }
    last_id = client.get_last_id();
    now = Instant::now();
    while now.elapsed() < time {
        generate_and_send_txs(
            &mut client,
            &clients,
            &id,
            &keypairs,
            &leader,
            txs,
            &mut last_id,
            threads,
            true,
        );
    }

    // Stop the sampling threads so it will collect the stats
    signal.store(true, Ordering::Relaxed);
    for t in v_threads {
        t.join().unwrap();
    }

    // Compute/report stats
    let mut max_of_maxes = 0.0;
    let mut total_txs = 0;
    for (max, txs) in maxes.read().unwrap().iter() {
        if *max > max_of_maxes {
            max_of_maxes = *max;
        }
        total_txs += *txs;
    }
    println!(
        "\nHighest TPS: {:.2} sampling period {}s total transactions: {} clients: {}",
        max_of_maxes,
        sample_period,
        total_txs,
        maxes.read().unwrap().len()
    );

    // join the crdt client threads
    for t in c_threads {
        t.join().unwrap();
    }
}
