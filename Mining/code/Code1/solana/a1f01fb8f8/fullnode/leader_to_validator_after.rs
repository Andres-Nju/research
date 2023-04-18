    fn leader_to_validator(&mut self) -> Result<()> {
        // TODO: We can avoid building the bank again once RecordStage is
        // integrated with BankingStage
        let (bank, entry_height, _) = Self::new_bank_from_ledger(&self.ledger_path);
        self.bank = Arc::new(bank);

        {
            let mut wcrdt = self.crdt.write().unwrap();
            let scheduled_leader = wcrdt.get_scheduled_leader(entry_height);
            match scheduled_leader {
                //TODO: Handle the case where we don't know who the next
                //scheduled leader is
                None => (),
                Some(leader_id) => wcrdt.set_leader(leader_id),
            }
        }

        // Make a new RPU to serve requests out of the new bank we've created
        // instead of the old one
        if !self.rpu.is_none() {
            let old_rpu = self.rpu.take().unwrap();
            old_rpu.close()?;
            self.rpu = Some(Rpu::new(
                &self.bank,
                self.requests_socket
                    .try_clone()
                    .expect("Failed to clone requests socket"),
                self.respond_socket
                    .try_clone()
                    .expect("Failed to clone respond socket"),
            ));
        }

        let tvu = Tvu::new(
            self.keypair.clone(),
            &self.bank,
            entry_height,
            self.crdt.clone(),
            self.shared_window.clone(),
            self.replicate_socket
                .iter()
                .map(|s| s.try_clone().expect("Failed to clone replicate sockets"))
                .collect(),
            self.repair_socket
                .try_clone()
                .expect("Failed to clone repair socket"),
            self.retransmit_socket
                .try_clone()
                .expect("Failed to clone retransmit socket"),
            Some(&self.ledger_path),
            self.exit.clone(),
        );
        let validator_state = ValidatorServices::new(tvu);
        self.node_role = Some(NodeRole::Validator(validator_state));
        Ok(())
    }
