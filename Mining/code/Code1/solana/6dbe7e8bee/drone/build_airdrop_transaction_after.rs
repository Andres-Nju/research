    pub fn build_airdrop_transaction(
        &mut self,
        req: DroneRequest,
    ) -> Result<Transaction, io::Error> {
        trace!("build_airdrop_transaction: {:?}", req);
        match req {
            DroneRequest::GetAirdrop {
                lamports,
                to,
                blockhash,
            } => {
                if self.check_request_limit(lamports) {
                    self.request_current += lamports;
                    solana_metrics::submit(
                        influxdb::Point::new("drone")
                            .add_tag("op", influxdb::Value::String("airdrop".to_string()))
                            .add_field("request_amount", influxdb::Value::Integer(lamports as i64))
                            .add_field(
                                "request_current",
                                influxdb::Value::Integer(self.request_current as i64),
                            )
                            .to_owned(),
                    );

                    info!("Requesting airdrop of {} to {:?}", lamports, to);

                    let create_instruction = system_instruction::create_user_account(
                        &self.mint_keypair.pubkey(),
                        &to,
                        lamports,
                    );
                    let message = Message::new(vec![create_instruction]);
                    Ok(Transaction::new(&[&self.mint_keypair], message, blockhash))
                } else {
                    Err(Error::new(
                        ErrorKind::Other,
                        format!(
                            "token limit reached; req: {} current: {} cap: {}",
                            lamports, self.request_current, self.request_cap
                        ),
                    ))
                }
            }
        }
    }
