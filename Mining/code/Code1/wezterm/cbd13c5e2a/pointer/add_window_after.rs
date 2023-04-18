    pub fn add_window(&self, surface: &WlSurface, pending: &Arc<Mutex<PendingMouse>>) {
        let mut inner = self.inner.lock().unwrap();
        inner
            .surface_to_pending
            .insert(surface.as_ref().id(), Arc::clone(pending));
        if inner.active_surface_id == 0 {
            inner.active_surface_id = surface.as_ref().id();
        }
    }
