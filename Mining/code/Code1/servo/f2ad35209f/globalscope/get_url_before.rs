    pub fn get_url(&self) -> ServoUrl {
        if let Some(window) = self.downcast::<Window>() {
            return window.get_url();
        }
        if let Some(worker) = self.downcast::<WorkerGlobalScope>() {
            return worker.get_url().clone();
        }
        if let Some(worker) = self.downcast::<WorkletGlobalScope>() {
            // TODO: is this the right URL to return?
            return worker.base_url();
        }
        unreachable!();
    }
