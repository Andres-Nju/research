                fn attach(&self, element: &Element) -> EventListener {
                    let this = element.clone();
                    let callback = self.callback.clone();
                    let listener = move |
                        #[cfg(feature = "std_web")] event: $type,
                        #[cfg(feature = "web_sys")] event: &web_sys::Event
                    | {
                        #[cfg(feature = "web_sys")]
                        let event: WebSysType = JsValue::from(event).into();
                        callback.emit($convert(&this, event));
                    };
                    cfg_match! {
                        feature = "std_web" => EventListener(Some(element.add_event_listener(listener))),
                        feature = "web_sys" => ({
                            // We should only set passive event listeners for `touchstart` and `touchmove`.
                            // See here: https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener#Improving_scrolling_performance_with_passive_listeners
                            if $name == "touchstart" || $name == "touchmove" {
                                EventListener::new(&EventTarget::from(element.clone()), $name, listener)
                            } else {
                                let options = EventListenerOptions::enable_prevent_default();
                                EventListener::new_with_options(&EventTarget::from(element.clone()), $name, options, listener)
                            }
                        }),
                    }
                }
            }
