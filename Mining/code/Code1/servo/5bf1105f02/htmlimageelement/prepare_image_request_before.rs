    fn prepare_image_request(&self, url: &ServoUrl, src: &DOMString) {
        match self.image_request.get() {
            ImageRequestPhase::Pending => {
                if let Some(pending_url) = self.pending_request.borrow().parsed_url.clone() {
                    // Step 12.1
                    if pending_url == *url {
                        return
                    }
                }
            },
            ImageRequestPhase::Current => {
                let mut current_request = self.current_request.borrow_mut();
                let mut pending_request = self.pending_request.borrow_mut();
                // step 12.4, create a new "image_request"
                match (current_request.parsed_url.clone(), current_request.state) {
                    (Some(parsed_url), State::PartiallyAvailable) => {
                        // Step 12.2
                        if parsed_url == *url {
                            // 12.3 abort pending request
                            pending_request.image = None;
                            pending_request.parsed_url = None;
                            LoadBlocker::terminate(&mut pending_request.blocker);
                            // TODO: queue a task to restart animation, if restart-animation is set
                            return
                        }
                        self.image_request.set(ImageRequestPhase::Pending);
                        self.init_image_request(&mut pending_request, &url, &src);
                        self.fetch_image(&url);
                    },
                    (_, State::Broken) | (_, State::Unavailable) => {
                        // Step 12.5
                        self.init_image_request(&mut current_request, &url, &src);
                        self.fetch_image(&url);
                    },
                    (_, _) => {
                        // step 12.6
                        self.image_request.set(ImageRequestPhase::Pending);
                        self.init_image_request(&mut pending_request, &url, &src);
                        self.fetch_image(&url);
                    },
                }
            }
        }
    }
