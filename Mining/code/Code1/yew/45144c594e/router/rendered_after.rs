    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.router_agent.send(RouteRequest::GetCurrentRoute);
        }
    }
