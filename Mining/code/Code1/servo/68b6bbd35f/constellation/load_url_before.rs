    fn load_url(
        &mut self,
        top_level_browsing_context_id: TopLevelBrowsingContextId,
        source_id: PipelineId,
        load_data: LoadData,
        replace: bool,
    ) -> Option<PipelineId> {
        // Allow the embedder to handle the url itself
        let (chan, port) = ipc::channel().expect("Failed to create IPC channel!");
        let msg = (
            Some(top_level_browsing_context_id),
            EmbedderMsg::AllowNavigation(load_data.url.clone(), chan),
        );
        self.embedder_proxy.send(msg);
        if let Ok(false) = port.recv() {
            return None;
        }

        debug!("Loading {} in pipeline {}.", load_data.url, source_id);
        // If this load targets an iframe, its framing element may exist
        // in a separate script thread than the framed document that initiated
        // the new load. The framing element must be notified about the
        // requested change so it can update its internal state.
        //
        // If replace is true, the current entry is replaced instead of a new entry being added.
        let (browsing_context_id, opener) = match self.pipelines.get(&source_id) {
            Some(pipeline) => (pipeline.browsing_context_id, pipeline.opener),
            None => {
                warn!("Pipeline {} loaded after closure.", source_id);
                return None;
            },
        };
        let (window_size, pipeline_id, parent_pipeline_id, is_visible) =
            match self.browsing_contexts.get(&browsing_context_id) {
                Some(ctx) => (
                    ctx.size,
                    ctx.pipeline_id,
                    ctx.parent_pipeline_id,
                    ctx.is_visible,
                ),
                None => {
                    // This should technically never happen (since `load_url` is
                    // only called on existing browsing contexts), but we prefer to
                    // avoid `expect`s or `unwrap`s in `Constellation` to ward
                    // against future changes that might break things.
                    warn!(
                        "Pipeline {} loaded url in closed browsing context {}.",
                        source_id,
                        browsing_context_id,
                    );
                    return None;
                },
            };

        match parent_pipeline_id {
            Some(parent_pipeline_id) => {
                // Find the script thread for the pipeline containing the iframe
                // and issue an iframe load through there.
                let msg = ConstellationControlMsg::Navigate(
                    parent_pipeline_id,
                    browsing_context_id,
                    load_data,
                    replace,
                );
                let result = match self.pipelines.get(&parent_pipeline_id) {
                    Some(parent_pipeline) => parent_pipeline.event_loop.send(msg),
                    None => {
                        warn!(
                            "Pipeline {:?} child loaded after closure",
                            parent_pipeline_id
                        );
                        return None;
                    },
                };
                if let Err(e) = result {
                    self.handle_send_error(parent_pipeline_id, e);
                }
                None
            },
            None => {
                // Make sure no pending page would be overridden.
                for change in &self.pending_changes {
                    if change.browsing_context_id == browsing_context_id {
                        // id that sent load msg is being changed already; abort
                        return None;
                    }
                }

                if self.get_activity(source_id) == DocumentActivity::Inactive {
                    // Disregard this load if the navigating pipeline is not actually
                    // active. This could be caused by a delayed navigation (eg. from
                    // a timer) or a race between multiple navigations (such as an
                    // onclick handler on an anchor element).
                    return None;
                }

                // Being here means either there are no pending changes, or none of the pending
                // changes would be overridden by changing the subframe associated with source_id.

                // Create the new pipeline

                let replace = if replace {
                    Some(NeedsToReload::No(pipeline_id))
                } else {
                    None
                };

                let new_pipeline_id = PipelineId::new();
                let sandbox = IFrameSandboxState::IFrameUnsandboxed;
                // TODO(mandreyel): why is this false? Should we not inherit the
                // privacy of the (existing) browsing context?
                let is_private = false;
                self.new_pipeline(
                    new_pipeline_id,
                    browsing_context_id,
                    top_level_browsing_context_id,
                    None,
                    opener,
                    window_size,
                    load_data.clone(),
                    sandbox,
                    is_private,
                    is_visible,
                );
                self.add_pending_change(SessionHistoryChange {
                    top_level_browsing_context_id: top_level_browsing_context_id,
                    browsing_context_id: browsing_context_id,
                    new_pipeline_id: new_pipeline_id,
                    replace,
                    // `load_url` is always invoked on an existing browsing context.
                    new_browsing_context_info: None,
                });
                Some(new_pipeline_id)
            },
        }
    }
