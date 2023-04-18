	pub fn tick(&self) {
		let elapsed = self.last_tick.read().elapsed();
		if elapsed < Duration::from_secs(5) {
			return;
		}

		let (client_report, full_report) = {
			let mut last_report = self.last_report.lock();
			let full_report = self.target.report();
			let diffed = full_report.client_report.clone() - &*last_report;
			*last_report = full_report.client_report.clone();
			(diffed, full_report)
		};

		let Report {
			importing,
			chain_info,
			queue_info,
			cache_sizes,
			sync_info,
			..
		} = full_report;

		let rpc_stats = self.rpc_stats.as_ref();
		let snapshot_sync = sync_info.as_ref().map_or(false, |s| s.snapshot_sync) && self.snapshot.as_ref().map_or(false, |s|
			match s.status() {
				RestorationStatus::Ongoing { .. } | RestorationStatus::Initializing { .. } => true,
				_ => false,
			}
		);
		if !importing && !snapshot_sync && elapsed < Duration::from_secs(30) {
			return;
		}

		*self.last_tick.write() = Instant::now();

		let paint = |c: Style, t: String| match self.with_color && atty::is(atty::Stream::Stdout) {
			true => format!("{}", c.paint(t)),
			false => t,
		};

		info!(target: "import", "{}  {}  {}  {}",
			match importing {
				true => match snapshot_sync {
					false => format!("Syncing {} {}  {}  {}+{} Qed",
						paint(White.bold(), format!("{:>8}", format!("#{}", chain_info.best_block_number))),
						paint(White.bold(), format!("{}", chain_info.best_block_hash)),
						if self.target.executes_transactions() {
							format!("{} blk/s {} tx/s {} Mgas/s",
								paint(Yellow.bold(), format!("{:5.2}", (client_report.blocks_imported * 1000) as f64 / elapsed.as_milliseconds() as f64)),
								paint(Yellow.bold(), format!("{:6.1}", (client_report.transactions_applied * 1000) as f64 / elapsed.as_milliseconds() as f64)),
								paint(Yellow.bold(), format!("{:4}", (client_report.gas_processed / From::from(elapsed.as_milliseconds() * 1000)).low_u64()))
							)
						} else {
							format!("{} hdr/s",
								paint(Yellow.bold(), format!("{:6.1}", (client_report.blocks_imported * 1000) as f64 / elapsed.as_milliseconds() as f64))
							)
						},
						paint(Green.bold(), format!("{:5}", queue_info.unverified_queue_size)),
						paint(Green.bold(), format!("{:5}", queue_info.verified_queue_size))
					),
					true => {
						self.snapshot.as_ref().map_or(String::new(), |s|
							match s.status() {
								RestorationStatus::Ongoing { state_chunks, block_chunks, state_chunks_done, block_chunks_done } => {
									format!("Syncing snapshot {}/{}", state_chunks_done + block_chunks_done, state_chunks + block_chunks)
								},
								RestorationStatus::Initializing { chunks_done } => {
									format!("Snapshot initializing ({} chunks restored)", chunks_done)
								},
								_ => String::new(),
							}
						)
					},
				},
				false => String::new(),
			},
			match sync_info.as_ref() {
				Some(ref sync_info) => format!("{}{}/{} peers",
					match importing {
						true => format!("{}   ", paint(Green.bold(), format!("{:>8}", format!("#{}", sync_info.last_imported_block_number)))),
						false => match sync_info.last_imported_old_block_number {
							Some(number) => format!("{}   ", paint(Yellow.bold(), format!("{:>8}", format!("#{}", number)))),
							None => String::new(),
						}
					},
					paint(Cyan.bold(), format!("{:2}", sync_info.num_peers)),
					paint(Cyan.bold(), format!("{:2}", sync_info.max_peers)),
				),
				_ => String::new(),
			},
			cache_sizes.display(Blue.bold(), &paint),
			match rpc_stats {
				Some(ref rpc_stats) => format!(
					"RPC: {} conn, {} req/s, {} µs",
					paint(Blue.bold(), format!("{:2}", rpc_stats.sessions())),
					paint(Blue.bold(), format!("{:4}", rpc_stats.requests_rate())),
					paint(Blue.bold(), format!("{:4}", rpc_stats.approximated_roundtrip())),
				),
				_ => String::new(),
			},
		);
	}
