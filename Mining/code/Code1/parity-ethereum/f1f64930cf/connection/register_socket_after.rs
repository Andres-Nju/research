	pub fn register_socket<Host: Handler>(&self, reg: Token, event_loop: &mut EventLoop<Host>) -> io::Result<()> {
		if self.registered.compare_and_swap(false, true, AtomicOrdering::SeqCst) {
			return Ok(());
        }
		trace!(target: "network", "connection register; token={:?}", reg);
		if let Err(e) = event_loop.register(&self.socket, reg, self.interest, PollOpt::edge() /* | PollOpt::oneshot() */) { // TODO: oneshot is broken on windows
			trace!(target: "network", "Failed to register {:?}, {:?}", reg, e);
		}
		Ok(())
	}
