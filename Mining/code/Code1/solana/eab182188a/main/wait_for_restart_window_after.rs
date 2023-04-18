fn wait_for_restart_window(
    ledger_path: &Path,
    min_idle_time_in_minutes: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let sleep_interval = Duration::from_secs(5);
    let min_delinquency_percentage = 0.05;

    let min_idle_slots = (min_idle_time_in_minutes as f64 * 60. / DEFAULT_S_PER_SLOT) as Slot;

    let admin_client = admin_rpc_service::connect(&ledger_path);
    let rpc_addr = admin_rpc_service::runtime()
        .block_on(async move { admin_client.await?.rpc_addr().await })
        .map_err(|err| format!("Unable to get validator RPC address: {}", err))?;

    let rpc_client = match rpc_addr {
        None => return Err("RPC not available".into()),
        Some(rpc_addr) => RpcClient::new_socket(rpc_addr),
    };

    let identity = rpc_client.get_identity()?;
    println_name_value("Identity:", &identity.to_string());
    println_name_value(
        "Minimum Idle Time:",
        &format!(
            "{} slots (~{} minutes)",
            min_idle_slots, min_idle_time_in_minutes
        ),
    );

    let mut current_epoch = None;
    let mut leader_schedule = VecDeque::new();
    let mut restart_snapshot = None;

    let progress_bar = new_spinner_progress_bar();
    let monitor_start_time = SystemTime::now();
    loop {
        let snapshot_slot = rpc_client.get_snapshot_slot().ok();
        let epoch_info = rpc_client.get_epoch_info_with_commitment(CommitmentConfig::processed())?;
        let healthy = rpc_client.get_health().ok().is_some();
        let delinquent_stake_percentage = {
            let vote_accounts = rpc_client.get_vote_accounts()?;
            let current_stake: u64 = vote_accounts
                .current
                .iter()
                .map(|va| va.activated_stake)
                .sum();
            let delinquent_stake: u64 = vote_accounts
                .delinquent
                .iter()
                .map(|va| va.activated_stake)
                .sum();
            let total_stake = current_stake + delinquent_stake;
            delinquent_stake as f64 / total_stake as f64
        };

        if match current_epoch {
            None => true,
            Some(current_epoch) => current_epoch != epoch_info.epoch,
        } {
            progress_bar.set_message(&format!(
                "Fetching leader schedule for epoch {}...",
                epoch_info.epoch
            ));
            let first_slot_in_epoch = epoch_info.absolute_slot - epoch_info.slot_index;
            leader_schedule = rpc_client
                .get_leader_schedule(Some(first_slot_in_epoch))?
                .ok_or_else(|| {
                    format!(
                        "Unable to get leader schedule from slot {}",
                        first_slot_in_epoch
                    )
                })?
                .get(&identity.to_string())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|slot_index| first_slot_in_epoch.saturating_add(slot_index as u64))
                .collect::<VecDeque<_>>();
            current_epoch = Some(epoch_info.epoch);
        }

        let status = {
            if !healthy {
                style("Node is unhealthy").red().to_string()
            } else {
                // Wait until a hole in the leader schedule before restarting the node
                let in_leader_schedule_hole =
                    if epoch_info.slot_index + min_idle_slots as u64 > epoch_info.slots_in_epoch {
                        Err("Current epoch is almost complete".to_string())
                    } else {
                        while leader_schedule
                            .get(0)
                            .map(|slot_index| *slot_index < epoch_info.absolute_slot)
                            .unwrap_or(false)
                        {
                            leader_schedule.pop_front();
                        }
                        match leader_schedule.get(0) {
                            None => {
                                Ok(()) // Validator has no leader slots
                            }
                            Some(next_leader_slot) => {
                                let idle_slots =
                                    next_leader_slot.saturating_sub(epoch_info.absolute_slot);
                                if idle_slots >= min_idle_slots {
                                    Ok(())
                                } else {
                                    Err(format!(
                                        "Validator will be leader soon. Next leader slot is {}",
                                        next_leader_slot
                                    ))
                                }
                            }
                        }
                    };

                match in_leader_schedule_hole {
                    Ok(_) => {
                        if restart_snapshot == None {
                            restart_snapshot = snapshot_slot;
                        }

                        if restart_snapshot == snapshot_slot {
                            "Waiting for a new snapshot".to_string()
                        } else if delinquent_stake_percentage >= min_delinquency_percentage {
                            style("Delinquency too high").red().to_string()
                        } else {
                            break; // Restart!
                        }
                    }
                    Err(why) => style(why).yellow().to_string(),
                }
            }
        };

        progress_bar.set_message(&format!(
            "{} | Processed Slot: {} | Snapshot Slot: {} | {:.2}% delinquent stake | {}",
            {
                let elapsed =
                    chrono::Duration::from_std(monitor_start_time.elapsed().unwrap()).unwrap();

                format!(
                    "{:02}:{:02}:{:02}",
                    elapsed.num_hours(),
                    elapsed.num_minutes() % 60,
                    elapsed.num_seconds() % 60
                )
            },
            epoch_info.absolute_slot,
            snapshot_slot
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string()),
            delinquent_stake_percentage * 100.,
            status
        ));
        std::thread::sleep(sleep_interval);
    }
    drop(progress_bar);
    println!("{}", style("Ready to restart").green());
    Ok(())
}
