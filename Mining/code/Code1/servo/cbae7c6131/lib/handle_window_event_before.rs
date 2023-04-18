    fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Idle => {
            }

            WindowEvent::Refresh => {
                self.compositor.composite();
            }

            WindowEvent::Resize => {
                self.compositor.on_resize_window_event();
            }

            WindowEvent::LoadUrl(top_level_browsing_context_id, url) => {
                let msg = ConstellationMsg::LoadUrl(top_level_browsing_context_id, url);
                if let Err(e) = self.constellation_chan.send(msg) {
                    warn!("Sending load url to constellation failed ({}).", e);
                }
            }

            WindowEvent::MouseWindowEventClass(mouse_window_event) => {
                self.compositor.on_mouse_window_event_class(mouse_window_event);
            }

            WindowEvent::MouseWindowMoveEventClass(cursor) => {
                self.compositor.on_mouse_window_move_event_class(cursor);
            }

            WindowEvent::Touch(event_type, identifier, location) => {
                self.compositor.on_touch_event(event_type, identifier, location);
            }

            WindowEvent::Scroll(delta, cursor, phase) => {
                self.compositor.on_scroll_event(delta, cursor, phase);
            }

            WindowEvent::Zoom(magnification) => {
                self.compositor.on_zoom_window_event(magnification);
            }

            WindowEvent::ResetZoom => {
                self.compositor.on_zoom_reset_window_event();
            }

            WindowEvent::PinchZoom(magnification) => {
                self.compositor.on_pinch_zoom_window_event(magnification);
            }

            WindowEvent::Navigation(top_level_browsing_context_id, direction) => {
                let msg = ConstellationMsg::TraverseHistory(top_level_browsing_context_id, direction);
                if let Err(e) = self.constellation_chan.send(msg) {
                    warn!("Sending navigation to constellation failed ({}).", e);
                }
            }

            WindowEvent::KeyEvent(ch, key, state, modifiers) => {
                let msg = ConstellationMsg::KeyEvent(ch, key, state, modifiers);
                if let Err(e) = self.constellation_chan.send(msg) {
                    warn!("Sending key event to constellation failed ({}).", e);
                }
            }

            WindowEvent::Quit => {
                self.compositor.maybe_start_shutting_down();
            }

            WindowEvent::Reload(top_level_browsing_context_id) => {
                let msg = ConstellationMsg::Reload(top_level_browsing_context_id);
                if let Err(e) = self.constellation_chan.send(msg) {
                    warn!("Sending reload to constellation failed ({}).", e);
                }
            }

            WindowEvent::ToggleWebRenderDebug(option) => {
                self.compositor.toggle_webrender_debug(option);
            }

            WindowEvent::CaptureWebRender => {
                self.compositor.capture_webrender();
            }

            WindowEvent::NewBrowser(url, response_chan) => {
                let msg = ConstellationMsg::NewBrowser(url, response_chan);
                if let Err(e) = self.constellation_chan.send(msg) {
                    warn!("Sending NewBrowser message to constellation failed ({}).", e);
                }
            }

            WindowEvent::SelectBrowser(ctx) => {
                let msg = ConstellationMsg::SelectBrowser(ctx);
                if let Err(e) = self.constellation_chan.send(msg) {
                    warn!("Sending SelectBrowser message to constellation failed ({}).", e);
                }
            }

            WindowEvent::CloseBrowser(ctx) => {
                let msg = ConstellationMsg::CloseBrowser(ctx);
                if let Err(e) = self.constellation_chan.send(msg) {
                    warn!("Sending CloseBrowser message to constellation failed ({}).", e);
                }
            }

            WindowEvent::SendError(ctx, e) => {
                let msg = ConstellationMsg::SendError(ctx, e);
                if let Err(e) = self.constellation_chan.send(msg) {
                    warn!("Sending CloseBrowser message to constellation failed ({}).", e);
                }
            }
        }
    }
