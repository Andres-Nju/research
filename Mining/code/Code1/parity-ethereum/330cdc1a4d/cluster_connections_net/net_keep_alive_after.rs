fn net_keep_alive(data: Arc<NetConnectionsData>) {
	let active_connections = data.active_connections();
	for connection in active_connections {
		// the last_message_time could change after active_connections() call
		// => we always need to call Instant::now() after getting last_message_time
		let last_message_time = connection.last_message_time();
		let now = Instant::now();
		let last_message_diff = now - last_message_time;
		if last_message_diff > KEEP_ALIVE_DISCONNECT_INTERVAL {
			warn!(target: "secretstore_net", "{}: keep alive timeout for node {}",
				data.self_key_pair.public(), connection.node_id());

			let node_id = *connection.node_id();
			if data.remove(&*connection) {
				let maintain_action = data.trigger.lock().on_connection_closed(&node_id);
				maintain_connection_trigger(data.clone(), maintain_action);
			}
			data.message_processor.process_disconnect(&node_id);
		}
		else if last_message_diff > KEEP_ALIVE_SEND_INTERVAL {
			connection.send_message(Message::Cluster(ClusterMessage::KeepAlive(message::KeepAlive {})));
		}
	}
}
