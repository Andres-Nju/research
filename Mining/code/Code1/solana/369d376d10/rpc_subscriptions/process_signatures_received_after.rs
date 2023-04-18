    fn process_signatures_received(
        (received_slot, signatures): &(Slot, Vec<Signature>),
        signature_subscriptions: &Arc<RpcSignatureSubscriptions>,
        notifier: &RpcNotifier,
    ) {
        for signature in signatures {
            if let Some(hashmap) = signature_subscriptions.read().unwrap().get(signature) {
                for (
                    _,
                    SubscriptionData {
                        sink,
                        config: is_received_notification_enabled,
                        ..
                    },
                ) in hashmap.iter()
                {
                    if is_received_notification_enabled.unwrap_or_default() {
                        notifier.notify(
                            Response {
                                context: RpcResponseContext {
                                    slot: *received_slot,
                                },
                                value: RpcSignatureResult::ReceivedSignature(
                                    ReceivedSignatureResult::ReceivedSignature,
                                ),
                            },
                            &sink,
                        );
                    }
                }
            }
        }
    }
