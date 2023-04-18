    fn bank(&self, commitment: Option<CommitmentConfig>) -> Arc<Bank> {
        debug!("RPC commitment_config: {:?}", commitment);
        let r_bank_forks = self.bank_forks.read().unwrap();

        let commitment_level = match commitment {
            None => CommitmentLevel::Max,
            Some(config) => config.commitment,
        };

        if commitment_level == CommitmentLevel::SingleGossip {
            let bank = self
                .optimistically_confirmed_bank
                .read()
                .unwrap()
                .bank
                .clone();
            debug!("RPC using optimistically confirmed slot: {:?}", bank.slot());
            return bank;
        }

        let slot = self
            .block_commitment_cache
            .read()
            .unwrap()
            .slot_with_commitment(commitment_level);

        match commitment_level {
            CommitmentLevel::Recent => {
                debug!("RPC using the heaviest slot: {:?}", slot);
            }
            CommitmentLevel::Root => {
                debug!("RPC using node root: {:?}", slot);
            }
            CommitmentLevel::Single => {
                debug!("RPC using confirmed slot: {:?}", slot);
            }
            CommitmentLevel::Max => {
                debug!("RPC using block: {:?}", slot);
            }
            CommitmentLevel::SingleGossip => unreachable!(),
        };

        r_bank_forks.get(slot).cloned().unwrap_or_else(|| {
            // We log an error instead of returning an error, because all known error cases
            // are due to known bugs that should be fixed instead.
            //
            // The slot may not be found as a result of a known bug in snapshot creation, where
            // the bank at the given slot was not included in the snapshot.
            // Also, it may occur after an old bank has been purged from BankForks and a new
            // BlockCommitmentCache has not yet arrived. To make this case impossible,
            // BlockCommitmentCache should hold an `Arc<Bank>` everywhere it currently holds
            // a slot.
            //
            // For more information, see https://github.com/solana-labs/solana/issues/11078
            error!(
                "Bank with {:?} not found at slot: {:?}",
                commitment_level, slot
            );
            r_bank_forks.root_bank().clone()
        })
    }
