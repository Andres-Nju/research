    fn ElementFromPoint(&self, x: Finite<f64>, y: Finite<f64>) -> Option<Root<Element>> {
        let x = *x as f32;
        let y = *y as f32;
        let point = &Point2D { x: x, y: y };
        let window = window_from_node(self);
        let viewport = window.window_size().unwrap().visible_viewport;

        if x < 0.0 || y < 0.0 || x > viewport.width.get() || y > viewport.height.get() {
            return None;
        }

        let js_runtime = unsafe { JS_GetRuntime(window.get_cx()) };

        match self.window.hit_test_query(*point, false) {
            Some(untrusted_node_address) => {
                let node = node::from_untrusted_node_address(js_runtime, untrusted_node_address);
                let parent_node = node.GetParentNode().unwrap();
                let element_ref = node.downcast::<Element>().unwrap_or_else(|| {
                    parent_node.downcast::<Element>().unwrap()
                });

                Some(Root::from_ref(element_ref))
            },
            None => self.GetDocumentElement()
        }
    }
