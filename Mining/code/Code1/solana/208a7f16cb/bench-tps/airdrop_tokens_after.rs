fn airdrop_tokens(client: &mut ThinClient, leader: &NodeInfo, id: &Keypair, tx_count: i64) {
    let mut drone_addr = leader.contact_info.tpu;
    drone_addr.set_port(DRONE_PORT);

    let starting_balance = client.poll_get_balance(&id.pubkey()).unwrap_or(0);
    metrics_submit_token_balance(starting_balance);

    if starting_balance < tx_count {
        let airdrop_amount = tx_count - starting_balance;
        println!(
            "Airdropping {:?} tokens from {}",
            airdrop_amount, drone_addr
        );

        let previous_balance = starting_balance;
        request_airdrop(&drone_addr, &id.pubkey(), airdrop_amount as u64).unwrap();

        // TODO: return airdrop Result from Drone instead of polling the
        //       network
        let mut current_balance = previous_balance;
        for _ in 0..20 {
            sleep(Duration::from_millis(500));
            current_balance = client.poll_get_balance(&id.pubkey()).unwrap();
            if starting_balance != current_balance {
                break;
            }
            println!(".");
        }
        metrics_submit_token_balance(current_balance);
        if current_balance - starting_balance != airdrop_amount {
            println!("Airdrop failed!");
            exit(1);
        }
    }
}
