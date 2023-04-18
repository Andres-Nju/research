    pub fn execute(&mut self, cx: &mut Context) -> CargoResult<()> {
        let _p = profile::start("executing the job graph");

        // We need to give a handle to the send half of our message queue to the
        // jobserver helper thrad. Unfortunately though we need the handle to be
        // `'static` as that's typically what's required when spawning a
        // thread!
        //
        // To work around this we transmute the `Sender` to a static lifetime.
        // we're only sending "longer living" messages and we should also
        // destroy all references to the channel before this function exits as
        // the destructor for the `helper` object will ensure the associated
        // thread i sno longer running.
        //
        // As a result, this `transmute` to a longer lifetime should be safe in
        // practice.
        let tx = self.tx.clone();
        let tx = unsafe {
            mem::transmute::<Sender<Message<'a>>, Sender<Message<'static>>>(tx)
        };
        let helper = cx.jobserver.clone().into_helper_thread(move |token| {
            drop(tx.send(Message::Token(token)));
        }).chain_err(|| {
            "failed to create helper thread for jobserver management"
        })?;

        crossbeam::scope(|scope| {
            self.drain_the_queue(cx, scope, &helper)
        })
    }
