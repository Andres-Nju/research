    fn logs_subscribe(
        &self,
        _meta: Self::Metadata,
        subscriber: Subscriber<RpcResponse<RpcLogsResponse>>,
        filter: RpcTransactionLogsFilter,
        config: RpcTransactionLogsConfig,
    ) {
        info!("logs_subscribe");

        let (address, include_votes) = match filter {
            RpcTransactionLogsFilter::All => (None, false),
            RpcTransactionLogsFilter::AllWithVotes => (None, true),
            RpcTransactionLogsFilter::Mentions(addresses) => {
                match addresses.len() {
                    1 => match param::<Pubkey>(&addresses[0], "mentions") {
                        Ok(address) => (Some(address), false),
                        Err(e) => {
                            subscriber.reject(e).unwrap();
                            return;
                        }
                    },
                    _ => {
                        // Room is reserved in the API to support multiple addresses, but for now
                        // the implementation only supports one
                        subscriber
                            .reject(Error {
                                code: ErrorCode::InvalidParams,
                                message: "Invalid Request: Only 1 address supported".into(),
                                data: None,
                            })
                            .unwrap();
                        return;
                    }
                }
            }
        };

        let id = self.uid.fetch_add(1, atomic::Ordering::Relaxed);
        let sub_id = SubscriptionId::Number(id as u64);
        self.subscriptions.add_logs_subscription(
            address,
            include_votes,
            config.commitment,
            sub_id,
            subscriber,
        )
    }
