fn loop_turn(
    pool: &ThreadPool,
    task_sender: &Sender<Task>,
    libdata_sender: &Sender<LibraryData>,
    connection: &Connection,
    world_state: &mut WorldState,
    loop_state: &mut LoopState,
    event: Event,
) -> Result<()> {
    let loop_start = Instant::now();

    // NOTE: don't count blocking select! call as a loop-turn time
    let _p = profile("main_loop_inner/loop-turn");
    log::info!("loop turn = {:?}", event);
    let queue_count = pool.queued_count();
    if queue_count > 0 {
        log::info!("queued count = {}", queue_count);
    }

    match event {
        Event::Task(task) => {
            on_task(task, &connection.sender, &mut loop_state.pending_requests, world_state);
            world_state.maybe_collect_garbage();
        }
        Event::Vfs(task) => {
            world_state.vfs.write().handle_task(task);
        }
        Event::Lib(lib) => {
            world_state.add_lib(lib);
            world_state.maybe_collect_garbage();
            loop_state.in_flight_libraries -= 1;
        }
        Event::CheckWatcher(task) => on_check_task(pool, task, world_state, task_sender)?,
        Event::Msg(msg) => match msg {
            Message::Request(req) => on_request(
                world_state,
                &mut loop_state.pending_requests,
                pool,
                task_sender,
                &connection.sender,
                loop_start,
                req,
            )?,
            Message::Notification(not) => {
                on_notification(
                    &connection.sender,
                    world_state,
                    &mut loop_state.pending_requests,
                    &mut loop_state.subscriptions,
                    not,
                )?;
            }
            Message::Response(resp) => {
                let removed = loop_state.pending_responses.remove(&resp.id);
                if !removed {
                    log::error!("unexpected response: {:?}", resp)
                }
            }
        },
    };

    let mut state_changed = false;
    if let Some(changes) = world_state.process_changes() {
        state_changed = true;
        loop_state.pending_libraries.extend(changes);
    }

    let max_in_flight_libs = pool.max_count().saturating_sub(2).max(1);
    while loop_state.in_flight_libraries < max_in_flight_libs
        && !loop_state.pending_libraries.is_empty()
    {
        let (root, files) = loop_state.pending_libraries.pop().unwrap();
        loop_state.in_flight_libraries += 1;
        let sender = libdata_sender.clone();
        pool.execute(move || {
            log::info!("indexing {:?} ... ", root);
            let _p = profile(&format!("indexed {:?}", root));
            let data = LibraryData::prepare(root, files);
            sender.send(data).unwrap();
        });
    }

    if !loop_state.workspace_loaded
        && world_state.roots_to_scan == 0
        && loop_state.pending_libraries.is_empty()
        && loop_state.in_flight_libraries == 0
    {
        loop_state.workspace_loaded = true;
        let n_packages: usize = world_state.workspaces.iter().map(|it| it.n_packages()).sum();
        if world_state.feature_flags().get("notifications.workspace-loaded") {
            let msg = format!("workspace loaded, {} rust packages", n_packages);
            show_message(req::MessageType::Info, msg, &connection.sender);
        }
        world_state.check_watcher.update();
    }

    if state_changed {
        update_file_notifications_on_threadpool(
            pool,
            world_state.snapshot(),
            world_state.options.publish_decorations,
            task_sender.clone(),
            loop_state.subscriptions.subscriptions(),
        )
    }

    let loop_duration = loop_start.elapsed();
    if loop_duration > Duration::from_millis(10) {
        log::error!("overly long loop turn: {:?}", loop_duration);
        if env::var("RA_PROFILE").is_ok() {
            show_message(
                req::MessageType::Error,
                format!("overly long loop turn: {:?}", loop_duration),
                &connection.sender,
            );
        }
    }

    Ok(())
}
