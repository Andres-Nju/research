fn send_barrier_transaction(barrier_client: &mut ThinClient, last_id: &mut Hash, id: &Keypair) {
    let transfer_start = Instant::now();

    let mut poll_count = 0;
    loop {
        if poll_count > 0 && poll_count % 8 == 0 {
            println!(
                "polling for barrier transaction confirmation, attempt {}",
                poll_count
            );
        }

        *last_id = barrier_client.get_last_id();
        let signature = barrier_client
            .transfer(0, &id, id.pubkey(), last_id)
            .expect("Unable to send barrier transaction");

        let confirmatiom = barrier_client.poll_for_signature(&signature);
        let duration_ms = duration_as_ms(&transfer_start.elapsed());
        if confirmatiom.is_ok() {
            println!("barrier transaction confirmed in {}ms", duration_ms);

            metrics::submit(
                influxdb::Point::new("bench-tps")
                    .add_tag(
                        "op",
                        influxdb::Value::String("send_barrier_transaction".to_string()),
                    )
                    .add_field("poll_count", influxdb::Value::Integer(poll_count))
                    .add_field("duration", influxdb::Value::Integer(duration_ms as i64))
                    .to_owned(),
            );

            // Sanity check that the client balance is still 1
            let balance = barrier_client
                .poll_balance_with_timeout(
                    &id.pubkey(),
                    &Duration::from_millis(100),
                    &Duration::from_secs(10),
                )
                .expect("Failed to get balance");
            if balance != 1 {
                panic!("Expected an account balance of 1 (balance: {}", balance);
            }
            break;
        }

        // Timeout after 3 minutes.  When running a CPU-only leader+validator+drone+bench-tps on a dev
        // machine, some batches of transactions can take upwards of 1 minute...
        if duration_ms > 1000 * 60 * 3 {
            println!("Error: Couldn't confirm barrier transaction!");
            exit(1);
        }

        let new_last_id = barrier_client.get_last_id();
        if new_last_id == *last_id {
            if poll_count > 0 && poll_count % 8 == 0 {
                println!("last_id is not advancing, still at {:?}", *last_id);
            }
        } else {
            *last_id = new_last_id;
        }

        poll_count += 1;
    }
}
