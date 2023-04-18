pub fn cluster_info_scale() {
    use solana_measure::measure::Measure;
    use solana_perf::test_tx::test_tx;
    use solana_runtime::bank::Bank;
    use solana_runtime::genesis_utils::{
        create_genesis_config_with_vote_accounts, ValidatorVoteKeypairs,
    };
    solana_logger::setup();
    let exit = Arc::new(AtomicBool::new(false));
    let num_nodes: usize = std::env::var("NUM_NODES")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .expect("could not parse NUM_NODES as a number");

    let vote_keypairs: Vec<_> = (0..num_nodes)
        .map(|_| ValidatorVoteKeypairs::new_rand())
        .collect();
    let genesis_config_info = create_genesis_config_with_vote_accounts(10_000, &vote_keypairs, 100);
    let bank0 = Bank::new(&genesis_config_info.genesis_config);
    let bank_forks = Arc::new(RwLock::new(BankForks::new(0, bank0)));

    let nodes: Vec<_> = vote_keypairs
        .into_iter()
        .map(|keypairs| test_node_with_bank(keypairs.node_keypair, &exit, bank_forks.clone()))
        .collect();
    let ci0 = nodes[0].0.my_contact_info();
    for node in &nodes[1..] {
        node.0.insert_info(ci0.clone());
    }

    let mut time = Measure::start("time");
    let mut done;
    let mut success = false;
    for _ in 0..30 {
        done = true;
        for (i, node) in nodes.iter().enumerate() {
            warn!("node {} peers: {}", i, node.0.gossip_peers().len());
            if node.0.gossip_peers().len() != num_nodes - 1 {
                done = false;
                break;
            }
        }
        if done {
            success = true;
            break;
        }
        sleep(Duration::from_secs(1));
    }
    time.stop();
    warn!("found {} nodes in {} success: {}", num_nodes, time, success);

    for num_votes in 1..1000 {
        let mut time = Measure::start("votes");
        let tx = test_tx();
        warn!("tx.message.account_keys: {:?}", tx.message.account_keys);
        nodes[0].0.push_vote(0, tx.clone());
        let mut success = false;
        for _ in 0..(30 * 5) {
            let mut not_done = 0;
            let mut num_old = 0;
            let mut num_push_total = 0;
            let mut num_pushes = 0;
            let mut num_pulls = 0;
            let mut num_inserts = 0;
            for node in nodes.iter() {
                //if node.0.get_votes(0).1.len() != (num_nodes * num_votes) {
                let has_tx = node
                    .0
                    .get_votes(0)
                    .1
                    .iter()
                    .filter(|v| v.message.account_keys == tx.message.account_keys)
                    .count();
                num_old += node.0.gossip.read().unwrap().push.num_old;
                num_push_total += node.0.gossip.read().unwrap().push.num_total;
                num_pushes += node.0.gossip.read().unwrap().push.num_pushes;
                num_pulls += node.0.gossip.read().unwrap().pull.num_pulls;
                num_inserts += node.0.gossip.read().unwrap().crds.num_inserts;
                if has_tx == 0 {
                    not_done += 1;
                }
            }
            warn!("not_done: {}/{}", not_done, nodes.len());
            warn!("num_old: {}", num_old);
            warn!("num_push_total: {}", num_push_total);
            warn!("num_pushes: {}", num_pushes);
            warn!("num_pulls: {}", num_pulls);
            warn!("num_inserts: {}", num_inserts);
            success = not_done < (nodes.len() / 20);
            if success {
                break;
            }
            sleep(Duration::from_millis(200));
        }
        time.stop();
        warn!(
            "propagated vote {} in {} success: {}",
            num_votes, time, success
        );
        sleep(Duration::from_millis(200));
        for node in nodes.iter() {
            node.0.gossip.write().unwrap().push.num_old = 0;
            node.0.gossip.write().unwrap().push.num_total = 0;
            node.0.gossip.write().unwrap().push.num_pushes = 0;
            node.0.gossip.write().unwrap().pull.num_pulls = 0;
            node.0.gossip.write().unwrap().crds.num_inserts = 0;
        }
    }

    exit.store(true, Ordering::Relaxed);
    for node in nodes {
        node.1.join().unwrap();
    }
}
