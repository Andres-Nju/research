    fn rendered(&mut self, _first_render: bool) {
        self.router_agent.send(RouteRequest::GetCurrentRoute);
    }
