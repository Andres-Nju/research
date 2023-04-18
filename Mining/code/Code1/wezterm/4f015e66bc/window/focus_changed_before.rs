    fn focus_changed(&mut self, focused: bool) {
        log::trace!("focus_changed {focused}, flagging geometry as unsure");
        self.sure_about_geometry = false;
        if self.has_focus != Some(focused) {
            self.has_focus.replace(focused);
            self.update_ime_position();
            log::trace!("Calling focus_change({focused})");
            self.events.dispatch(WindowEvent::FocusChanged(focused));
        }

        // This is a bit gross; in <https://github.com/wez/wezterm/issues/2063>
        // we observe that CONFIGURE_NOTIFY isn't being sent around certain
        // WM operations when nvidia drivers are in used.
        // However, focus events are in the right ballpark, but still happen
        // before the new geometry is applied.
        // This schedules an invalidation of both our sense of geometry and
        // the window a short time after the focus event is processed in the
        // hope that it can observe the changed window properties and update
        // without the human needing to interact with the window.
        let delay = self.config.focus_change_repaint_delay;
        if delay != 0 {
            let window_id = self.window_id;
            promise::spawn::spawn(async move {
                async_io::Timer::after(std::time::Duration::from_millis(delay)).await;
                XConnection::with_window_inner(window_id, |inner| {
                    inner.sure_about_geometry = false;
                    inner.invalidate();
                    Ok(())
                });
            })
            .detach();
        }
    }
