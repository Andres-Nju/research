	fn message(&self, _io: &IoContext<NetworkIoMessage>, message: &NetworkIoMessage) {
		if let NetworkIoMessage::NetworkStarted(ref public_url) = *message {
			let mut url = self.public_url.write();
			if url.as_ref().map_or(true, |uref| uref != public_url) {
				info!(target: "network", "Public node URL: {}", Colour::White.bold().paint(AsRef::<str>::as_ref(public_url)));
			}
			*url = Some(public_url.to_owned());
		}
	}
