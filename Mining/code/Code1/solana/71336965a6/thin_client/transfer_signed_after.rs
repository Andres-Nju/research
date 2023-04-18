    pub fn transfer_signed(&self, tx: &Transaction) -> io::Result<Signature> {
        let data = serialize(&tx).expect("serialize Transaction in pub fn transfer_signed");
        assert!(data.len() < PACKET_DATA_SIZE);
        self.transactions_socket
            .send_to(&data, &self.transactions_addr)?;
        Ok(tx.signatures[0])
    }
