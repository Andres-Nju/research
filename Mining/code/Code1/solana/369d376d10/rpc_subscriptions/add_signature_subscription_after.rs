    pub fn add_signature_subscription(
        &self,
        signature: Signature,
        signature_subscribe_config: Option<RpcSignatureSubscribeConfig>,
        sub_id: SubscriptionId,
        subscriber: Subscriber<Response<RpcSignatureResult>>,
    ) {
        let (commitment, enable_received_notification) = signature_subscribe_config
            .map(|config| (config.commitment, config.enable_received_notification))
            .unwrap_or_default();

        let commitment_level = commitment
            .unwrap_or_else(CommitmentConfig::recent)
            .commitment;

        let mut subscriptions = if commitment_level == CommitmentLevel::SingleGossip {
            self.subscriptions
                .gossip_signature_subscriptions
                .write()
                .unwrap()
        } else {
            self.subscriptions.signature_subscriptions.write().unwrap()
        };
        add_subscription(
            &mut subscriptions,
            signature,
            commitment,
            sub_id,
            subscriber,
            0, // last_notified_slot is not utilized for signature subscriptions
            enable_received_notification,
        );
    }
