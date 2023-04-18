pub fn run<F>(future: F)
where
  F: Future<Item = (), Error = ()> + Send + 'static,
{
  // tokio::runtime::current_thread::run(future)
  let rt = create_threadpool_runtime();
  rt.block_on_all(future).unwrap();
}

/// THIS IS A HACK AND SHOULD BE AVOIDED.
///
/// This creates a new tokio runtime, with many new threads, to execute the
/// given future. This is useful when we want to block the main runtime to
/// resolve a future without worrying that we'll us up all the threads in the
/// main runtime.
pub fn block_on<F, R, E>(future: F) -> Result<R, E>
where
  F: Send + 'static + Future<Item = R, Error = E>,
  R: Send + 'static,
  E: Send + 'static,
{
  use std::sync::mpsc::channel;
  use std::thread;
  let (sender, receiver) = channel();
  // Create a new runtime to evaluate the future asynchronously.
  thread::spawn(move || {
    let mut rt = create_threadpool_runtime();
    let r = rt.block_on(future);
    sender.send(r).unwrap();
  });
  receiver.recv().unwrap()
}

// Set the default executor so we can use tokio::spawn(). It's difficult to
// pass around mut references to the runtime, so using with_default is
// preferable. Ideally Tokio would provide this function.
#[cfg(test)]
pub fn init<F>(f: F)
where
  F: FnOnce(),
{
  let rt = create_threadpool_runtime();
  let mut executor = rt.executor();
  let mut enter = tokio_executor::enter().expect("Multiple executors at once");
  tokio_executor::with_default(&mut executor, &mut enter, move |_enter| f());
}
