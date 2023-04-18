    fn submit_mining_proof(&self) {
        // No point if we've got no storage account...
        assert!(
            self.client
                .poll_get_balance(&self.storage_keypair.pubkey())
                .unwrap()
                > 0
        );
        // ...or no lamports for fees
        assert!(
            self.client
                .poll_get_balance(&self.keypair.pubkey())
                .unwrap()
                > 0
        );

        let (blockhash, _) = self
            .client
            .get_recent_blockhash()
            .expect("No recent blockhash");
        let instruction = storage_instruction::mining_proof(
            &self.storage_keypair.pubkey(),
            self.hash,
            self.slot,
            Signature::new(&self.signature.to_bytes()),
        );
        let message = Message::new_with_payer(vec![instruction], Some(&self.keypair.pubkey()));
        let mut transaction = Transaction::new(
            &[self.keypair.as_ref(), self.storage_keypair.as_ref()],
            message,
            blockhash,
        );
        self.client
            .send_and_confirm_transaction(
                &[&self.keypair, &self.storage_keypair],
                &mut transaction,
                10,
                0,
            )
            .expect("transfer didn't work!");
    }
