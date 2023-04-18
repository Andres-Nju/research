    fn run(self: Box<Self>) {
        let mut state = self.state.borrow_mut();
        if state.destroyed {
            return;
        }
        match self.event {
            AgentLifecycleEvent::Create(link) => {
                state.agent = Some(AGN::create(link));
            }
            AgentLifecycleEvent::Message(msg) => {
                state
                    .agent
                    .as_mut()
                    .expect("agent was not created to process messages")
                    .update(msg);
            }
            AgentLifecycleEvent::Connected(id) => {
                state
                    .agent
                    .as_mut()
                    .expect("agent was not created to send a connected message")
                    .connected(id);
            }
            AgentLifecycleEvent::Input(inp, id) => {
                state
                    .agent
                    .as_mut()
                    .expect("agent was not created to process inputs")
                    .handle_input(inp, id);
            }
            AgentLifecycleEvent::Disconnected(id) => {
                state
                    .agent
                    .as_mut()
                    .expect("agent was not created to send a disconnected message")
                    .disconnected(id);
            }
            AgentLifecycleEvent::Destroy => {
                let mut agent = state
                    .agent
                    .take()
                    .expect("trying to destroy not existent agent");
                agent.destroy();
            }
        }
    }
