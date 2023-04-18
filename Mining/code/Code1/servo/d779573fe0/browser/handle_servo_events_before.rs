    pub fn handle_servo_events(&mut self, events: Vec<(Option<BrowserId>, EmbedderMsg)>) {
        for (browser_id, msg) in events {
            match msg {
                EmbedderMsg::Status(status) => {
                    self.status = status;
                },
                EmbedderMsg::ChangePageTitle(title) => {
                    self.title = title;

                    let fallback_title: String = if let Some(ref current_url) = self.current_url {
                        current_url.to_string()
                    } else {
                        String::from("Untitled")
                    };
                    let title = match self.title {
                        Some(ref title) if title.len() > 0 => &**title,
                        _ => &fallback_title,
                    };
                    let title = format!("{} - Servo", title);
                    self.window.set_title(&title);
                },
                EmbedderMsg::MoveTo(point) => {
                    self.window.set_position(point);
                },
                EmbedderMsg::ResizeTo(size) => {
                    self.window.set_inner_size(size);
                },
                EmbedderMsg::Alert(message, sender) => {
                    if !opts::get().headless {
                        let _ = thread::Builder::new()
                            .name("display alert dialog".to_owned())
                            .spawn(move || {
                                tinyfiledialogs::message_box_ok(
                                    "Alert!",
                                    &message,
                                    MessageBoxIcon::Warning,
                                );
                            })
                            .unwrap()
                            .join()
                            .expect("Thread spawning failed");
                    }
                    if let Err(e) = sender.send(()) {
                        let reason = format!("Failed to send Alert response: {}", e);
                        self.event_queue
                            .push(WindowEvent::SendError(browser_id, reason));
                    }
                },
                EmbedderMsg::AllowUnload(sender) => {
                    // Always allow unload for now.
                    if let Err(e) = sender.send(true) {
                        let reason = format!("Failed to send AllowUnload response: {}", e);
                        self.event_queue
                            .push(WindowEvent::SendError(browser_id, reason));
                    }
                },
                EmbedderMsg::AllowNavigationRequest(pipeline_id, _url) => {
                    if let Some(_browser_id) = browser_id {
                        self.event_queue
                            .push(WindowEvent::AllowNavigationResponse(pipeline_id, true));
                    }
                },
                EmbedderMsg::AllowOpeningBrowser(response_chan) => {
                    // Note: would be a place to handle pop-ups config.
                    // see Step 7 of #the-rules-for-choosing-a-browsing-context-given-a-browsing-context-name
                    if let Err(e) = response_chan.send(true) {
                        warn!("Failed to send AllowOpeningBrowser response: {}", e);
                    };
                },
                EmbedderMsg::BrowserCreated(new_browser_id) => {
                    // TODO: properly handle a new "tab"
                    self.browsers.push(new_browser_id);
                    if self.browser_id.is_none() {
                        self.browser_id = Some(new_browser_id);
                    }
                    self.event_queue
                        .push(WindowEvent::SelectBrowser(new_browser_id));
                },
                EmbedderMsg::Keyboard(key_event) => {
                    self.handle_key_from_servo(browser_id, key_event);
                },
                EmbedderMsg::GetClipboardContents(sender) => {
                    let contents = match self.clipboard_ctx {
                        Some(ref mut ctx) => {
                            match ctx.get_contents() {
                                Ok(c) => c,
                                Err(e) => {
                                    warn!("Error getting clipboard contents ({}), defaulting to empty string", e);
                                    "".to_owned()
                                },
                            }
                        },
                        None => "".to_owned(),
                    };
                    if let Err(e) = sender.send(contents) {
                        warn!("Failed to send clipboard ({})", e);
                    }
                }
                EmbedderMsg::SetClipboardContents(text) => {
                    if let Some(ref mut ctx) = self.clipboard_ctx {
                        if let Err(e) = ctx.set_contents(text) {
                            warn!("Error setting clipboard contents ({})", e);
                        }
                    }
                }
                EmbedderMsg::SetCursor(cursor) => {
                    self.window.set_cursor(cursor);
                },
                EmbedderMsg::NewFavicon(url) => {
                    self.favicon = Some(url);
                },
                EmbedderMsg::HeadParsed => {
                    self.loading_state = Some(LoadingState::Loading);
                },
                EmbedderMsg::HistoryChanged(urls, current) => {
                    self.current_url = Some(urls[current].clone());
                },
                EmbedderMsg::SetFullscreenState(state) => {
                    self.window.set_fullscreen(state);
                },
                EmbedderMsg::LoadStart => {
                    self.loading_state = Some(LoadingState::Connecting);
                },
                EmbedderMsg::LoadComplete => {
                    self.loading_state = Some(LoadingState::Loaded);
                },
                EmbedderMsg::CloseBrowser => {
                    // TODO: close the appropriate "tab".
                    let _ = self.browsers.pop();
                    if let Some(prev_browser_id) = self.browsers.last() {
                        self.browser_id = Some(*prev_browser_id);
                        self.event_queue
                            .push(WindowEvent::SelectBrowser(*prev_browser_id));
                    } else {
                        self.event_queue.push(WindowEvent::Quit);
                    }
                },
                EmbedderMsg::Shutdown => {
                    self.shutdown_requested = true;
                },
                EmbedderMsg::Panic(_reason, _backtrace) => {},
                EmbedderMsg::GetSelectedBluetoothDevice(devices, sender) => {
                    let selected = platform_get_selected_devices(devices);
                    if let Err(e) = sender.send(selected) {
                        let reason =
                            format!("Failed to send GetSelectedBluetoothDevice response: {}", e);
                        self.event_queue.push(WindowEvent::SendError(None, reason));
                    };
                },
                EmbedderMsg::SelectFiles(patterns, multiple_files, sender) => {
                    let res = match (
                        opts::get().headless,
                        get_selected_files(patterns, multiple_files),
                    ) {
                        (true, _) | (false, None) => sender.send(None),
                        (false, Some(files)) => sender.send(Some(files)),
                    };
                    if let Err(e) = res {
                        let reason = format!("Failed to send SelectFiles response: {}", e);
                        self.event_queue.push(WindowEvent::SendError(None, reason));
                    };
                },
                EmbedderMsg::ShowIME(_kind) => {
                    debug!("ShowIME received");
                },
                EmbedderMsg::HideIME => {
                    debug!("HideIME received");
                },
                EmbedderMsg::ReportProfile(bytes) => {
                    let filename = env::var("PROFILE_OUTPUT").unwrap_or("samples.json".to_string());
                    let result = File::create(&filename).and_then(|mut f| f.write_all(&bytes));
                    if let Err(e) = result {
                        error!("Failed to store profile: {}", e);
                    }
                },
            }
        }
    }
