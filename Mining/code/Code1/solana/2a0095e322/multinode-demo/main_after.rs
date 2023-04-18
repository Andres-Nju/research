fn main() {
    let mut threads = 4usize;
    let mut num_nodes = 10usize;
    let mut leader = "leader.json".to_string();
    let mut client_addr: SocketAddr = "127.0.0.1:8010".parse().unwrap();

    let mut opts = Options::new();
    opts.optopt("l", "", "leader", "leader.json");
    opts.optopt("c", "", "client address", "host");
    opts.optopt("t", "", "number of threads", &format!("{}", threads));
    opts.optopt(
        "n",
        "",
        "number of nodes to converge to",
        &format!("{}", num_nodes),
    );
    opts.optflag("h", "help", "print help");
    let args: Vec<String> = env::args().collect();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    if matches.opt_present("h") {
        let program = args[0].clone();
        print_usage(&program, opts);
        return;
    }
    if matches.opt_present("l") {
        leader = matches.opt_str("l").unwrap();
    }
    if matches.opt_present("c") {
        client_addr = matches.opt_str("c").unwrap().parse().unwrap();
    }
    if matches.opt_present("t") {
        threads = matches.opt_str("t").unwrap().parse().expect("integer");
    }
    if matches.opt_present("n") {
        num_nodes = matches.opt_str("n").unwrap().parse().expect("integer");
    }

    let leader: ReplicatedData = read_leader(leader);
    let signal = Arc::new(AtomicBool::new(false));
    let mut c_threads = vec![];
    let validators = converge(
        &client_addr,
        &leader,
        signal.clone(),
        num_nodes + 2,
        &mut c_threads,
    );

    if stdin_isatty() {
        eprintln!("nothing found on stdin, expected a json file");
        exit(1);
    }

    let mut buffer = String::new();
    let num_bytes = stdin().read_to_string(&mut buffer).unwrap();
    if num_bytes == 0 {
        eprintln!("empty file on stdin, expected a json file");
        exit(1);
    }

    println!("Parsing stdin...");
    let demo: MintDemo = serde_json::from_str(&buffer).unwrap_or_else(|e| {
        eprintln!("failed to parse json: {}", e);
        exit(1);
    });
    let mut client = mk_client(&client_addr, &leader);

    println!("Get last ID...");
    let last_id = client.get_last_id().wait().unwrap();
    println!("Got last ID {:?}", last_id);

    let rnd = GenKeys::new(demo.mint.keypair().public_key_bytes());

    println!("Creating keypairs...");
    let txs = demo.num_accounts / 2;
    let keypairs = rnd.gen_n_keypairs(demo.num_accounts);
    let keypair_pairs: Vec<_> = keypairs.chunks(2).collect();

    println!("Signing transactions...");
    let now = Instant::now();
    let transactions: Vec<_> = keypair_pairs
        .into_par_iter()
        .map(|chunk| Transaction::new(&chunk[0], chunk[1].pubkey(), 1, last_id))
        .collect();
    let mut duration = now.elapsed();
    let ns = duration.as_secs() * 1_000_000_000 + u64::from(duration.subsec_nanos());
    let bsps = txs as f64 / ns as f64;
    let nsps = ns as f64 / txs as f64;
    println!(
        "Done. {} thousand signatures per second, {}us per signature",
        bsps * 1_000_000_f64,
        nsps / 1_000_f64
    );

    let initial_tx_count = client.transaction_count();
    println!("initial count {}", initial_tx_count);

    println!("Transfering {} transactions in {} batches", txs, threads);
    let now = Instant::now();
    let sz = transactions.len() / threads;
    let chunks: Vec<_> = transactions.chunks(sz).collect();
    chunks.into_par_iter().for_each(|trs| {
        println!("Transferring 1 unit {} times... to", trs.len());
        let client = mk_client(&client_addr, &leader);
        for tr in trs {
            client.transfer_signed(tr.clone()).unwrap();
        }
    });

    println!("Waiting for transactions to complete...",);
    for _ in 0..10 {
        let mut tx_count = client.transaction_count();
        duration = now.elapsed();
        let txs = tx_count - initial_tx_count;
        println!("Transactions processed {}", txs);
        let ns = duration.as_secs() * 1_000_000_000 + u64::from(duration.subsec_nanos());
        let tps = (txs * 1_000_000_000) as f64 / ns as f64;
        println!("{} tps", tps);
        sleep(Duration::new(1, 0));
    }
    for val in validators {
        let mut client = mk_client(&client_addr, &val);
        let mut tx_count = client.transaction_count();
        duration = now.elapsed();
        let txs = tx_count - initial_tx_count;
        println!("Transactions processed {} on {}", txs, val.events_addr);
        let ns = duration.as_secs() * 1_000_000_000 + u64::from(duration.subsec_nanos());
        let tps = (txs * 1_000_000_000) as f64 / ns as f64;
        println!("{} tps on {}", tps, val.events_addr);
    }
    signal.store(true, Ordering::Relaxed);
    for t in c_threads {
        t.join().unwrap();
    }
}

