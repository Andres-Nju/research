    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\
             Creation time: {}\n\
             Cluster type: {:?}\n\
             Genesis hash: {}\n\
             Shred version: {}\n\
             Ticks per slot: {:?}\n\
             Hashes per tick: {:?}\n\
             Target tick duration: {:?}\n\
             Slots per epoch: {}\n\
             Warmup epochs: {}abled\n\
             Slots per year: {}\n\
             {:?}\n\
             {:?}\n\
             {:?}\n\
             Capitalization: {} SOL in {} accounts\n\
             Native instruction processors: {:#?}\n\
             Rewards pool: {:#?}\n\
             ",
            Utc.timestamp_opt(self.creation_time, 0)
                .unwrap()
                .to_rfc3339(),
            self.cluster_type,
            self.hash(),
            compute_shred_version(&self.hash(), None),
            self.ticks_per_slot,
            self.poh_config.hashes_per_tick,
            self.poh_config.target_tick_duration,
            self.epoch_schedule.slots_per_epoch,
            if self.epoch_schedule.warmup {
                "en"
            } else {
                "dis"
            },
            self.slots_per_year(),
            self.inflation,
            self.rent,
            self.fee_rate_governor,
            lamports_to_sol(
                self.accounts
                    .iter()
                    .map(|(pubkey, account)| {
                        assert!(account.lamports > 0, "{:?}", (pubkey, account));
                        account.lamports
                    })
                    .sum::<u64>()
            ),
            self.accounts.len(),
            self.native_instruction_processors,
            self.rewards_pools,
        )
    }
