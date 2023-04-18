    fn dispatcher() -> Dispatcher<Self>;
}

/// A newtype around a bridge to indicate that it is distinct from a normal bridge
pub struct Dispatcher<T>(Box<dyn Bridge<T>>);

impl<T> fmt::Debug for Dispatcher<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Dispatcher<_>")
    }
}

impl<T> Deref for Dispatcher<T> {
    type Target = dyn Bridge<T>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl<T> DerefMut for Dispatcher<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

/// Marker trait to indicate which Discoverers are able to be used with dispatchers.
pub trait Dispatchable: Discoverer {}

/// Implements rules to register a worker in a separate thread.
pub trait Threaded {
    /// Executes an agent in the current environment.
    /// Uses in `main` function of a worker.
    fn register();
}

impl<T> Threaded for T
where
    T: Agent<Reach = Public>,
{
    fn register() {
        let scope = AgentScope::<T>::new();
        let responder = WorkerResponder {};
        let link = AgentLink::connect(&scope, responder);
        let upd = AgentUpdate::Create(link);
        scope.send(upd);
        let handler = move |data: Vec<u8>| {
            let msg = ToWorker::<T::Input>::unpack(&data);
            match msg {
                ToWorker::Connected(id) => {
                    let upd = AgentUpdate::Connected(id);
                    scope.send(upd);
                }
                ToWorker::ProcessInput(id, value) => {
                    let upd = AgentUpdate::Input(value, id);
                    scope.send(upd);
                }
                ToWorker::Disconnected(id) => {
                    let upd = AgentUpdate::Disconnected(id);
                    scope.send(upd);
                }
                ToWorker::Destroy => {
                    let upd = AgentUpdate::Destroy;
                    scope.send(upd);
                    js! {
                        // Terminates web worker
                        self.close();
                    };
                }
            }
        };
        let loaded: FromWorker<T::Output> = FromWorker::WorkerLoaded;
        let loaded = loaded.pack();
        js! {
            var handler = @{handler};
            self.onmessage = function(event) {
                handler(event.data);
            };
            self.postMessage(@{loaded});
        };
    }
}

impl<T> Bridged for T
where
    T: Agent,
{
    fn bridge(callback: Callback<Self::Output>) -> Box<dyn Bridge<Self>> {
        Self::Reach::spawn_or_join(Some(callback))
    }
}

impl<T> Dispatched for T
where
    T: Agent,
    <T as Agent>::Reach: Dispatchable,
{
    fn dispatcher() -> Dispatcher<T> {
        Dispatcher(Self::Reach::spawn_or_join::<T>(None))
    }
}

/// Determine a visibility of an agent.
#[doc(hidden)]
pub trait Discoverer {
    /// Spawns an agent and returns `Bridge` implementation.
    fn spawn_or_join<AGN: Agent>(_callback: Option<Callback<AGN::Output>>) -> Box<dyn Bridge<AGN>> {
        unimplemented!();
    }
}

/// Bridge to a specific kind of worker.
pub trait Bridge<AGN: Agent> {
    /// Send a message to an agent.
    fn send(&mut self, msg: AGN::Input);
}

// <<< SAME THREAD >>>

struct LocalAgent<AGN: Agent> {
    scope: AgentScope<AGN>,
    slab: SharedOutputSlab<AGN>,
}

type Last = bool;

impl<AGN: Agent> LocalAgent<AGN> {
    pub fn new(scope: &AgentScope<AGN>) -> Self {
        let slab = Rc::new(RefCell::new(Slab::new()));
        LocalAgent {
            scope: scope.clone(),
            slab,
        }
    }

    fn slab(&self) -> SharedOutputSlab<AGN> {
        self.slab.clone()
    }

    fn create_bridge(&mut self, callback: Option<Callback<AGN::Output>>) -> ContextBridge<AGN> {
        let respondable = callback.is_some();
        let id: usize = self.slab.borrow_mut().insert(callback);
        let id = HandlerId::new(id, respondable);
        ContextBridge {
            scope: self.scope.clone(),
            id,
        }
    }

    fn remove_bridge(&mut self, bridge: &ContextBridge<AGN>) -> Last {
        let mut slab = self.slab.borrow_mut();
        let _ = slab.remove(bridge.id.raw_id());
        slab.is_empty()
    }
}

thread_local! {
    static LOCAL_AGENTS_POOL: RefCell<AnyMap> = RefCell::new(AnyMap::new());
}

/// Create a single instance in the current thread.
pub struct Context;

impl Discoverer for Context {
    fn spawn_or_join<AGN: Agent>(callback: Option<Callback<AGN::Output>>) -> Box<dyn Bridge<AGN>> {
        let mut scope_to_init = None;
        let bridge = LOCAL_AGENTS_POOL.with(|pool| {
            match pool.borrow_mut().entry::<LocalAgent<AGN>>() {
                Entry::Occupied(mut entry) => {
                    // TODO Insert callback!
                    entry.get_mut().create_bridge(callback)
                }
                Entry::Vacant(entry) => {
                    let scope = AgentScope::<AGN>::new();
                    let launched = LocalAgent::new(&scope);
                    let responder = SlabResponder {
                        slab: launched.slab(),
                    };
                    scope_to_init = Some((scope.clone(), responder));
                    entry.insert(launched).create_bridge(callback)
                }
            }
        });
        if let Some((scope, responder)) = scope_to_init {
            let agent_link = AgentLink::connect(&scope, responder);
            let upd = AgentUpdate::Create(agent_link);
            scope.send(upd);
        }
        let upd = AgentUpdate::Connected(bridge.id);
        bridge.scope.send(upd);
        Box::new(bridge)
    }
}

impl Dispatchable for Context {}

struct SlabResponder<AGN: Agent> {
    slab: Shared<Slab<Option<Callback<AGN::Output>>>>,
}

impl<AGN: Agent> Responder<AGN> for SlabResponder<AGN> {
    fn response(&self, id: HandlerId, output: AGN::Output) {
        locate_callback_and_respond::<AGN>(&self.slab, id, output);
    }
}

/// The slab contains the callback, the id is used to look up the callback,
/// and the output is the message that will be sent via the callback.
fn locate_callback_and_respond<AGN: Agent>(
    slab: &SharedOutputSlab<AGN>,
    id: HandlerId,
    output: AGN::Output,
) {
    match slab.borrow().get(id.raw_id()).cloned() {
        Some(Some(callback)) => callback.emit(output),
        Some(None) => warn!("The Id of the handler: {}, while present in the slab, is not associated with a callback.", id.raw_id()),
        None => warn!("Id of handler does not exist in the slab: {}.", id.raw_id()),
    }
}

struct ContextBridge<AGN: Agent> {
    scope: AgentScope<AGN>,
    id: HandlerId,
}

impl<AGN: Agent> Bridge<AGN> for ContextBridge<AGN> {
    fn send(&mut self, msg: AGN::Input) {
        let upd = AgentUpdate::Input(msg, self.id);
        self.scope.send(upd);
    }
}

impl<AGN: Agent> Drop for ContextBridge<AGN> {
    fn drop(&mut self) {
        LOCAL_AGENTS_POOL.with(|pool| {
            let terminate_worker = {
                if let Some(launched) = pool.borrow_mut().get_mut::<LocalAgent<AGN>>() {
                    launched.remove_bridge(self)
                } else {
                    false
                }
            };

            let upd = AgentUpdate::Disconnected(self.id);
            self.scope.send(upd);

            if terminate_worker {
                let upd = AgentUpdate::Destroy;
                self.scope.send(upd);
                pool.borrow_mut().remove::<LocalAgent<AGN>>();
            }
        });
    }
}

/// Create an instance in the current thread.
pub struct Job;

impl Discoverer for Job {
    fn spawn_or_join<AGN: Agent>(callback: Option<Callback<AGN::Output>>) -> Box<dyn Bridge<AGN>> {
        let callback = callback.expect("Callback required for Job");
        let scope = AgentScope::<AGN>::new();
        let responder = CallbackResponder { callback };
        let agent_link = AgentLink::connect(&scope, responder);
        let upd = AgentUpdate::Create(agent_link);
        scope.send(upd);
        let upd = AgentUpdate::Connected(SINGLETON_ID);
        scope.send(upd);
        let bridge = JobBridge { scope };
        Box::new(bridge)
    }
}

const SINGLETON_ID: HandlerId = HandlerId(0, true);

struct CallbackResponder<AGN: Agent> {
    callback: Callback<AGN::Output>,
}

impl<AGN: Agent> Responder<AGN> for CallbackResponder<AGN> {
    fn response(&self, id: HandlerId, output: AGN::Output) {
        assert_eq!(id.raw_id(), SINGLETON_ID.raw_id());
        self.callback.emit(output);
    }
}

struct JobBridge<AGN: Agent> {
    scope: AgentScope<AGN>,
}

impl<AGN: Agent> Bridge<AGN> for JobBridge<AGN> {
    fn send(&mut self, msg: AGN::Input) {
        let upd = AgentUpdate::Input(msg, SINGLETON_ID);
        self.scope.send(upd);
    }
}

impl<AGN: Agent> Drop for JobBridge<AGN> {
    fn drop(&mut self) {
        let upd = AgentUpdate::Disconnected(SINGLETON_ID);
        self.scope.send(upd);
        let upd = AgentUpdate::Destroy;
        self.scope.send(upd);
    }
}

// <<< SEPARATE THREAD >>>

/// Create a new instance for every bridge.
pub struct Private;

impl Discoverer for Private {
    fn spawn_or_join<AGN: Agent>(callback: Option<Callback<AGN::Output>>) -> Box<dyn Bridge<AGN>> {
        let callback = callback.expect("Callback required for Private agents");
        let handler = move |data: Vec<u8>| {
            let msg = FromWorker::<AGN::Output>::unpack(&data);
            match msg {
                FromWorker::WorkerLoaded => {
                    // TODO Send `Connected` message
                }
                FromWorker::ProcessOutput(id, output) => {
                    assert_eq!(id.raw_id(), SINGLETON_ID.raw_id());
                    callback.emit(output);
                }
            }
        };
        // TODO Need somethig better...
        let name_of_resource = AGN::name_of_resource();
        let worker = js! {
            var worker = new Worker(@{name_of_resource});
            var handler = @{handler};
            worker.onmessage = function(event) {
                handler(event.data);
            };
            return worker;
        };
        let bridge = PrivateBridge {
            worker,
            _agent: PhantomData,
        };
        Box::new(bridge)
    }
}

/// A connection manager for components interaction with workers.
pub struct PrivateBridge<T: Agent> {
    worker: Value,
    _agent: PhantomData<T>,
}

impl<AGN: Agent> Bridge<AGN> for PrivateBridge<AGN> {
    fn send(&mut self, msg: AGN::Input) {
        // TODO Important! Implement.
        // Use a queue to collect a messages if an instance is not ready
        // and send them to an agent when it will reported readiness.
        let msg = ToWorker::ProcessInput(SINGLETON_ID, msg).pack();
        let worker = &self.worker;
        js! {
            var worker = @{worker};
            var bytes = @{msg};
            worker.postMessage(bytes);
        };
    }
}

impl<AGN: Agent> Drop for PrivateBridge<AGN> {
    fn drop(&mut self) {
        // TODO Send `Destroy` message.
    }
}

struct RemoteAgent<AGN: Agent> {
    worker: Value,
    slab: SharedOutputSlab<AGN>,
}

impl<AGN: Agent> RemoteAgent<AGN> {
    pub fn new(worker: &Value, slab: SharedOutputSlab<AGN>) -> Self {
        RemoteAgent {
            worker: worker.clone(),
            slab,
        }
    }

    fn create_bridge(&mut self, callback: Option<Callback<AGN::Output>>) -> PublicBridge<AGN> {
        let respondable = callback.is_some();
        let id: usize = self.slab.borrow_mut().insert(callback);
        let id = HandlerId::new(id, respondable);
        PublicBridge {
            worker: self.worker.clone(),
            id,
            _agent: PhantomData,
        }
    }

    fn remove_bridge(&mut self, bridge: &PublicBridge<AGN>) -> Last {
        let mut slab = self.slab.borrow_mut();
        let _ = slab.remove(bridge.id.raw_id());
        slab.is_empty()
    }
}

thread_local! {
    static REMOTE_AGENTS_POOL: RefCell<AnyMap> = RefCell::new(AnyMap::new());
}

/// Create a single instance in a tab.
pub struct Public;

impl Discoverer for Public {
    fn spawn_or_join<AGN: Agent>(callback: Option<Callback<AGN::Output>>) -> Box<dyn Bridge<AGN>> {
        let bridge = REMOTE_AGENTS_POOL.with(|pool| {
            match pool.borrow_mut().entry::<RemoteAgent<AGN>>() {
                Entry::Occupied(mut entry) => {
                    // TODO Insert callback!
                    entry.get_mut().create_bridge(callback)
                }
                Entry::Vacant(entry) => {
                    let slab_base: Shared<Slab<Option<Callback<AGN::Output>>>> =
                        Rc::new(RefCell::new(Slab::new()));
                    let slab = slab_base.clone();
                    let handler = move |data: Vec<u8>| {
                        let msg = FromWorker::<AGN::Output>::unpack(&data);
                        match msg {
                            FromWorker::WorkerLoaded => {
                                // TODO Use `AtomicBool` lock to check its loaded
                                // TODO Send `Connected` message
                            }
                            FromWorker::ProcessOutput(id, output) => {
                                locate_callback_and_respond::<AGN>(&slab, id, output);
                            }
                        }
                    };
                    let name_of_resource = AGN::name_of_resource();
                    let worker = js! {
                        var worker = new Worker(@{name_of_resource});
                        var handler = @{handler};
                        worker.onmessage = function(event) {
                            handler(event.data);
                        };
                        return worker;
                    };
                    let launched = RemoteAgent::new(&worker, slab_base);
                    entry.insert(launched).create_bridge(callback)
                }
            }
        });
        Box::new(bridge)
    }
}

impl Dispatchable for Public {}

/// A connection manager for components interaction with workers.
pub struct PublicBridge<T: Agent> {
    worker: Value,
    id: HandlerId,
    _agent: PhantomData<T>,
}

fn send_to_remote<AGN: Agent>(worker: &Value, msg: ToWorker<AGN::Input>) {
    // TODO Important! Implement.
    // Use a queue to collect a messages if an instance is not ready
    // and send them to an agent when it will reported readiness.
    let msg = msg.pack();
    js! {
        var worker = @{worker};
        var bytes = @{msg};
        worker.postMessage(bytes);
    };
}

impl<AGN: Agent> Bridge<AGN> for PublicBridge<AGN> {
    fn send(&mut self, msg: AGN::Input) {
        let msg = ToWorker::ProcessInput(self.id, msg);
        send_to_remote::<AGN>(&self.worker, msg);
    }
}

impl<AGN: Agent> Drop for PublicBridge<AGN> {
    fn drop(&mut self) {
        REMOTE_AGENTS_POOL.with(|pool| {
            let terminate_worker = {
                if let Some(launched) = pool.borrow_mut().get_mut::<RemoteAgent<AGN>>() {
                    launched.remove_bridge(self)
                } else {
                    false
                }
            };
            let upd = ToWorker::Disconnected(self.id);
            send_to_remote::<AGN>(&self.worker, upd);
            if terminate_worker {
                let upd = ToWorker::Destroy;
                send_to_remote::<AGN>(&self.worker, upd);
                pool.borrow_mut().remove::<RemoteAgent<AGN>>();
            }
        });
    }
}

/// Create a single instance in a browser.
pub struct Global;

impl Discoverer for Global {}

/// Declares the behavior of the agent.
pub trait Agent: Sized + 'static {
    /// Reach capaility of the agent.
    type Reach: Discoverer;
    /// Type of an input messagae.
    type Message;
    /// Incoming message type.
    type Input: Transferable;
    /// Outgoing message type.
    type Output: Transferable;

    /// Creates an instance of an agent.
    fn create(link: AgentLink<Self>) -> Self;

    /// This method called on every update message.
    fn update(&mut self, msg: Self::Message);

    /// This method called on when a new bridge created.
    fn connected(&mut self, _id: HandlerId) {}

    /// This method called on every incoming message.
    fn handle(&mut self, msg: Self::Input, id: HandlerId);

    /// This method called on when a new bridge destroyed.
    fn disconnected(&mut self, _id: HandlerId) {}

    /// Creates an instance of an agent.
    fn destroy(&mut self) {}

    /// Represents the name of loading resorce for remote workers which
    /// have to live in a separate files.
    fn name_of_resource() -> &'static str {
        "main.js"
    }
}

/// This sctruct holds a reference to a component and to a global scheduler.
pub struct AgentScope<AGN: Agent> {
    shared_agent: Shared<AgentRunnable<AGN>>,
}

impl<AGN: Agent> Clone for AgentScope<AGN> {
    fn clone(&self) -> Self {
        AgentScope {
            shared_agent: self.shared_agent.clone(),
        }
    }
}

impl<AGN: Agent> AgentScope<AGN> {
    fn new() -> Self {
        let shared_agent = Rc::new(RefCell::new(AgentRunnable::new()));
        AgentScope { shared_agent }
    }

    fn send(&self, update: AgentUpdate<AGN>) {
        let envelope = AgentEnvelope {
            shared_agent: self.shared_agent.clone(),
            update,
        };
        let runnable: Box<dyn Runnable> = Box::new(envelope);
        scheduler().put_and_try_run(runnable);
    }
}

trait Responder<AGN: Agent> {
    fn response(&self, id: HandlerId, output: AGN::Output);
}

struct WorkerResponder {}

impl<AGN: Agent> Responder<AGN> for WorkerResponder {
    fn response(&self, id: HandlerId, output: AGN::Output) {
        let msg = FromWorker::ProcessOutput(id, output);
        let data = msg.pack();
        js! {
            var data = @{data};
            self.postMessage(data);
        };
    }
}

/// Link to agent's scope for creating callbacks.
pub struct AgentLink<AGN: Agent> {
    scope: AgentScope<AGN>,
    responder: Box<dyn Responder<AGN>>,
}

impl<AGN: Agent> AgentLink<AGN> {
    /// Create link for a scope.
    fn connect<T>(scope: &AgentScope<AGN>, responder: T) -> Self
    where
        T: Responder<AGN> + 'static,
    {
        AgentLink {
            scope: scope.clone(),
            responder: Box::new(responder),
        }
    }

    /// Send response to an actor.
    pub fn response(&self, id: HandlerId, output: AGN::Output) {
        self.responder.response(id, output);
    }

    /// This method sends messages back to the component's loop.
    pub fn send_back<F, IN>(&self, function: F) -> Callback<IN>
    where
        F: Fn(IN) -> AGN::Message + 'static,
    {
        let scope = self.scope.clone();
        let closure = move |input| {
            let output = function(input);
            let msg = AgentUpdate::Message(output);
            scope.clone().send(msg);
        };
        closure.into()
    }
}

impl<AGN: Agent> fmt::Debug for AgentLink<AGN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AgentLink<_>")
    }
}
