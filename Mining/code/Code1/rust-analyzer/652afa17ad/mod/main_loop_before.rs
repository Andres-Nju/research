pub fn main_loop(
    internal_mode: bool,
    root: PathBuf,
    msg_receriver: &Receiver<RawMessage>,
    msg_sender: &Sender<RawMessage>,
) -> Result<()> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .panic_handler(|_| error!("thread panicked :("))
        .build()
        .unwrap();
    let (task_sender, task_receiver) = unbounded::<Task>();
    let (fs_worker, fs_watcher) = vfs::roots_loader();
    let (ws_worker, ws_watcher) = workspace_loader();

    info!("server initialized, serving requests");
    let mut state = ServerWorldState::new();

    let mut pending_requests = FxHashSet::default();
    let mut subs = Subscriptions::new();
    let main_res = main_loop_inner(
        internal_mode,
        root,
        &pool,
        msg_sender,
        msg_receriver,
        task_sender,
        task_receiver.clone(),
        fs_worker,
        ws_worker,
        &mut state,
        &mut pending_requests,
        &mut subs,
    );

    info!("waiting for tasks to finish...");
    task_receiver.for_each(|task| on_task(task, msg_sender, &mut pending_requests));
    info!("...tasks have finished");
    info!("joining threadpool...");
    drop(pool);
    info!("...threadpool has finished");

    let fs_res = fs_watcher.stop();
    let ws_res = ws_watcher.stop();

    main_res?;
    fs_res?;
    ws_res?;

    Ok(())
}
