    fn split_gossip_messages(mut msgs: Vec<CrdsValue>) -> Vec<Vec<CrdsValue>> {
        let mut messages = vec![];
        while !msgs.is_empty() {
            let mut size = 0;
            let mut payload = vec![];
            while let Some(msg) = msgs.pop() {
                let msg_size = msg.size();
                if size + msg_size > MAX_PROTOCOL_PAYLOAD_SIZE as u64 {
                    if msg_size < MAX_PROTOCOL_PAYLOAD_SIZE as u64 {
                        msgs.push(msg);
                    } else {
                        debug!(
                            "dropping message larger than the maximum payload size {:?}",
                            msg
                        );
                    }
                    break;
                }
                size += msg_size;
                payload.push(msg);
            }
            messages.push(payload);
        }
        messages
    }
