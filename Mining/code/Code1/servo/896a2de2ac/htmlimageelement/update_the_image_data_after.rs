    pub fn update_the_image_data(&self) {
        let document = document_from_node(self);
        let window = document.window();
        let elem = self.upcast::<Element>();
        let src = elem.get_string_attribute(&local_name!("src"));
        let base_url = document.base_url();

        // https://html.spec.whatwg.org/multipage/#reacting-to-dom-mutations
        // Always first set the current request to unavailable,
        // ensuring img.complete is false.
        {
            let mut current_request = self.current_request.borrow_mut();
            current_request.state = State::Unavailable;
        }

        if !document.is_active() {
            // Step 1 (if the document is inactive)
            // TODO: use GlobalScope::enqueue_microtask,
            // to queue micro task to come back to this algorithm
        }
        // Step 2 abort if user-agent does not supports images
        // NOTE: Servo only supports images, skipping this step

        // Step 3, 4
        let mut selected_source = None;
        let mut pixel_density = None;
        let src_set = elem.get_string_attribute(&local_name!("srcset"));
        let is_parent_picture = elem
            .upcast::<Node>()
            .GetParentElement()
            .map_or(false, |p| p.is::<HTMLPictureElement>());
        if src_set.is_empty() && !is_parent_picture && !src.is_empty() {
            selected_source = Some(src.clone());
            pixel_density = Some(1 as f64);
        };

        // Step 5
        *self.last_selected_source.borrow_mut() = selected_source.clone();

        // Step 6, check the list of available images
        if !selected_source
            .as_ref()
            .map_or(false, |source| source.is_empty())
        {
            if let Ok(img_url) = base_url.join(&src) {
                let image_cache = window.image_cache();
                let response = image_cache.find_image_or_metadata(
                    img_url.clone().into(),
                    UsePlaceholder::No,
                    CanRequestImages::No,
                );
                if let Ok(ImageOrMetadataAvailable::ImageAvailable(image, url)) = response {
                    // Cancel any outstanding tasks that were queued before the src was
                    // set on this element.
                    self.generation.set(self.generation.get() + 1);
                    // Step 6.3
                    let metadata = ImageMetadata {
                        height: image.height,
                        width: image.width,
                    };
                    // Step 6.3.2 abort requests
                    self.abort_request(State::CompletelyAvailable, ImageRequestPhase::Current);
                    self.abort_request(State::Unavailable, ImageRequestPhase::Pending);
                    let mut current_request = self.current_request.borrow_mut();
                    current_request.final_url = Some(url);
                    current_request.image = Some(image.clone());
                    current_request.metadata = Some(metadata);
                    // Step 6.3.6
                    current_request.current_pixel_density = pixel_density;
                    let this = Trusted::new(self);
                    let src = String::from(src);
                    let _ = window.task_manager().dom_manipulation_task_source().queue(
                        task!(image_load_event: move || {
                            let this = this.root();
                            {
                                let mut current_request =
                                    this.current_request.borrow_mut();
                                current_request.parsed_url = Some(img_url);
                                current_request.source_url = Some(src.into());
                            }
                            // TODO: restart animation, if set.
                            this.upcast::<EventTarget>().fire_event(atom!("load"));
                        }),
                        window.upcast(),
                    );
                    return;
                }
            }
        }
        // step 7, await a stable state.
        self.generation.set(self.generation.get() + 1);
        let task = ImageElementMicrotask::StableStateUpdateImageDataTask {
            elem: DomRoot::from_ref(self),
            generation: self.generation.get(),
        };
        ScriptThread::await_stable_state(Microtask::ImageElement(task));
    }
