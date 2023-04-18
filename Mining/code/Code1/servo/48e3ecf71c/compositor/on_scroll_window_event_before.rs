    fn on_scroll_window_event(&mut self,
                              delta: TypedPoint2D<f32, DevicePixel>,
                              cursor: TypedPoint2D<i32, DevicePixel>) {
        let event_phase = match (self.scroll_in_progress, self.in_scroll_transaction) {
            (false, Some(last_scroll)) if last_scroll.elapsed() > Duration::from_millis(80) =>
                ScrollEventPhase::Start,
            (_, _) => ScrollEventPhase::Move(self.scroll_in_progress),
        };
        self.in_scroll_transaction = Some(Instant::now());
        self.pending_scroll_zoom_events.push(ScrollZoomEvent {
            magnification: 1.0,
            delta: delta,
            cursor: cursor,
            phase: event_phase,
            event_count: 1,
        });
    }
