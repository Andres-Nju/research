    fn drop(&mut self) {
        if let Err(err) = self.ipc_renderer.send(CanvasMsg::Common(CanvasCommonMsg::Close)) {
            warn!("Could not close canvas: {}", err)
        }
    }
