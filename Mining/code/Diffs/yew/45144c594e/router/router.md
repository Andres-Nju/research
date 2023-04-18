File_Code/yew/45144c594e/router/router_after.rs --- Rust
188     fn rendered(&mut self, _first_render: bool) {                                                                                                        188     fn rendered(&mut self, first_render: bool) {
...                                                                                                                                                          189         if first_render {
189         self.router_agent.send(RouteRequest::GetCurrentRoute);                                                                                           190             self.router_agent.send(RouteRequest::GetCurrentRoute);
190     }                                                                                                                                                    191         }

