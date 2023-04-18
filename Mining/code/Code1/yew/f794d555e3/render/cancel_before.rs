    fn cancel(&mut self) {
        let handle = self.0.take().expect("tried to cancel render twice");
        js! { @(no_return)
            var handle = @{handle};
            cancelAnimationFrame(handle.timeout_id);
            handle.callback.drop();
        }
    }
