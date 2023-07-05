// Copyright 2016 TiKV Project Authors. Licensed under Apache-2.0.

//! Scheduler which schedules the execution of `storage::Command`s.
//!
//! There is one scheduler for each store. It receives commands from clients, executes them against
//! the MVCC layer storage engine.
//!
//! Logically, the data organization hierarchy from bottom to top is row -> region -> store ->
//! database. But each region is replicated onto N stores for reliability, the replicas form a Raft
//! group, one of which acts as the leader. When the client read or write a row, the command is
//! sent to the scheduler which is on the region leader's store.
//!
//! Scheduler runs in a single-thread event loop, but command executions are delegated to a pool of
//! worker thread.
//!
//! Scheduler keeps track of all the running commands and uses latches to ensure serialized access
//! to the overlapping rows involved in concurrent commands. But note that scheduler only ensures
//! serialized access to the overlapping rows at command level, but a transaction may consist of
//! multiple commands, therefore conflicts may happen at transaction level. Transaction semantics
//! is ensured by the transaction protocol implemented in the client library, which is transparent
//! to the scheduler.

use spin::Mutex;
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::u64;

use kvproto::kvrpcpb::CommandPri;
use prometheus::HistogramTimer;
use tikv_util::collections::HashMap;

use crate::storage::kv::{with_tls_engine, Result as EngineResult};
use crate::storage::lock_manager::{
    self, store_wait_table_is_empty, DetectorScheduler, WaiterMgrScheduler,
};
use crate::storage::txn::latch::{Latches, Lock};
use crate::storage::txn::process::{execute_callback, Executor, MsgScheduler, ProcessResult, Task};
use crate::storage::txn::sched_pool::SchedPool;
use crate::storage::txn::Error;
use crate::storage::{metrics::*, Key};
use crate::storage::{Command, Engine, Error as StorageError, StorageCb};

const TASKS_SLOTS_NUM: usize = 1 << 12; // 4096 slots.

/// Message types for the scheduler event loop.
pub enum Msg {
    RawCmd {
        cmd: Command,
        cb: StorageCb,
    },
    ReadFinished {
        cid: u64,
        pr: ProcessResult,
        tag: CommandKind,
    },
    WriteFinished {
        cid: u64,
        pr: ProcessResult,
        result: EngineResult<()>,
        tag: CommandKind,
    },
    FinishedWithErr {
        cid: u64,
        err: Error,
        tag: CommandKind,
    },
    WaitForLock {
        cid: u64,
        start_ts: u64,
        pr: ProcessResult,
        lock: lock_manager::Lock,
        is_first_lock: bool,
    },
}

/// Debug for messages.
impl Debug for Msg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// Display for messages.
impl Display for Msg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Msg::RawCmd { ref cmd, .. } => write!(f, "RawCmd {:?}", cmd),
            Msg::ReadFinished { cid, .. } => write!(f, "ReadFinished [cid={}]", cid),
            Msg::WriteFinished { cid, .. } => write!(f, "WriteFinished [cid={}]", cid),
            Msg::FinishedWithErr { cid, .. } => write!(f, "FinishedWithErr [cid={}]", cid),
            Msg::WaitForLock { cid, .. } => write!(f, "WaitForLock [cid={}]", cid),
        }
    }
}

// It stores context of a task.
struct TaskContext {
    task: Option<Task>,

    lock: Lock,
    cb: StorageCb,
    write_bytes: usize,
    tag: CommandKind,
    // How long it waits on latches.
    latch_timer: Option<HistogramTimer>,
    // Total duration of a command.
    _cmd_timer: HistogramTimer,
}

impl TaskContext {
    fn new(task: Task, latches: &Latches, cb: StorageCb) -> TaskContext {
        let tag = task.cmd().tag();
        let lock = gen_command_lock(latches, task.cmd());
        // Write command should acquire write lock.
        if !task.cmd().readonly() && !lock.is_write_lock() {
            panic!("write lock is expected for command {:?}", task.cmd());
        }
        let write_bytes = if lock.is_write_lock() {
            task.cmd().write_bytes()
        } else {
            0
        };

        TaskContext {
            task: Some(task),
            lock,
            cb,
            write_bytes,
            tag,
            latch_timer: Some(SCHED_LATCH_HISTOGRAM_VEC.get(tag).start_coarse_timer()),
            _cmd_timer: SCHED_HISTOGRAM_VEC_STATIC.get(tag).start_coarse_timer(),
        }
    }

    fn on_schedule(&mut self) {
        self.latch_timer.take();
    }
}

struct SchedulerInner {
    // slot_id -> { cid -> `TaskContext` } in the slot.
    task_contexts: Vec<Mutex<HashMap<u64, TaskContext>>>,

    // cmd id generator
    id_alloc: AtomicU64,

    // write concurrency control
    latches: Latches,

    sched_pending_write_threshold: usize,

    // worker pool
    worker_pool: SchedPool,

    // high priority commands and system commands will be delivered to this pool
    high_priority_pool: SchedPool,

    // used to control write flow
    running_write_bytes: AtomicUsize,

    waiter_mgr_scheduler: Option<WaiterMgrScheduler>,

    detector_scheduler: Option<DetectorScheduler>,
}

#[inline]
fn id_index(cid: u64) -> usize {
    cid as usize % TASKS_SLOTS_NUM
}

impl SchedulerInner {
    /// Generates the next command ID.
    #[inline]
    fn gen_id(&self) -> u64 {
        let id = self.id_alloc.fetch_add(1, Ordering::Relaxed);
        id + 1
    }

    fn dequeue_task(&self, cid: u64) -> Task {
        let mut tasks = self.task_contexts[id_index(cid)].lock();
        let task = tasks.get_mut(&cid).unwrap().task.take().unwrap();
        assert_eq!(task.cid, cid);
        task
    }

    fn enqueue_task(&self, task: Task, callback: StorageCb) {
        let cid = task.cid;
        let tctx = TaskContext::new(task, &self.latches, callback);

        let running_write_bytes = self
            .running_write_bytes
            .fetch_add(tctx.write_bytes, Ordering::AcqRel) as i64;
        SCHED_WRITING_BYTES_GAUGE.set(running_write_bytes + tctx.write_bytes as i64);
        SCHED_CONTEX_GAUGE.inc();

        let mut tasks = self.task_contexts[id_index(cid)].lock();
        if tasks.insert(cid, tctx).is_some() {
            panic!("TaskContext cid={} shouldn't exist", cid);
        }
    }

    fn dequeue_task_context(&self, cid: u64) -> TaskContext {
        let tctx = self.task_contexts[id_index(cid)]
            .lock()
            .remove(&cid)
            .unwrap();

        let running_write_bytes = self
            .running_write_bytes
            .fetch_sub(tctx.write_bytes, Ordering::AcqRel) as i64;
        SCHED_WRITING_BYTES_GAUGE.set(running_write_bytes - tctx.write_bytes as i64);
        SCHED_CONTEX_GAUGE.dec();

        tctx
    }

    fn too_busy(&self) -> bool {
        fail_point!("txn_scheduler_busy", |_| true);
        self.running_write_bytes.load(Ordering::Acquire) >= self.sched_pending_write_threshold
    }

    /// Tries to acquire all the required latches for a command.
    ///
    /// Returns `true` if successful; returns `false` otherwise.
    fn acquire_lock(&self, cid: u64) -> bool {
        let mut task_contexts = self.task_contexts[id_index(cid)].lock();
        let tctx = task_contexts.get_mut(&cid).unwrap();
        if self.latches.acquire(&mut tctx.lock, cid) {
            tctx.on_schedule();
            return true;
        }
        false
    }
}

/// Scheduler which schedules the execution of `storage::Command`s.
#[derive(Clone)]
pub struct Scheduler<E: Engine> {
    // `engine` is `None` means currently the program is in scheduler worker threads.
    engine: Option<E>,
    inner: Arc<SchedulerInner>,
}

unsafe impl<E: Engine> Send for Scheduler<E> {}

impl<E: Engine> Scheduler<E> {
    /// Creates a scheduler.
    pub fn new(
        engine: E,
        waiter_mgr_scheduler: Option<WaiterMgrScheduler>,
        detector_scheduler: Option<DetectorScheduler>,
        concurrency: usize,
        worker_pool_size: usize,
        sched_pending_write_threshold: usize,
    ) -> Self {
        // Add 2 logs records how long is need to initialize TASKS_SLOTS_NUM * 2048000 `Mutex`es.
        // In a 3.5G Hz machine it needs 1.3s, which is a notable duration during start-up.
        info!("Scheduler::new is called to initialize the transaction scheduler");
        let mut task_contexts = Vec::with_capacity(TASKS_SLOTS_NUM);
        for _ in 0..TASKS_SLOTS_NUM {
            task_contexts.push(Mutex::new(Default::default()));
        }

        let inner = Arc::new(SchedulerInner {
            task_contexts,
            id_alloc: AtomicU64::new(0),
            latches: Latches::new(concurrency),
            running_write_bytes: AtomicUsize::new(0),
            sched_pending_write_threshold,
            worker_pool: SchedPool::new(engine.clone(), worker_pool_size, "sched-worker-pool"),
            high_priority_pool: SchedPool::new(
                engine.clone(),
                std::cmp::max(1, worker_pool_size / 2),
                "sched-high-pri-pool",
            ),
            waiter_mgr_scheduler,
            detector_scheduler,
        });

        info!("Scheduler::new is finished, the transaction scheduler is initialized");
        Scheduler {
            engine: Some(engine),
            inner,
        }
    }

    pub fn run_cmd(&self, cmd: Command, callback: StorageCb) {
        self.on_receive_new_cmd(cmd, callback);
    }
}

impl<E: Engine> Scheduler<E> {
    fn fetch_executor(&self, priority: CommandPri, is_sys_cmd: bool) -> Executor<E, Self> {
        let pool = if priority == CommandPri::High || is_sys_cmd {
            self.inner.high_priority_pool.clone()
        } else {
            self.inner.worker_pool.clone()
        };
        let scheduler = Scheduler {
            engine: None,
            inner: Arc::clone(&self.inner),
        };
        Executor::new(
            scheduler,
            pool,
            self.inner.waiter_mgr_scheduler.clone(),
            self.inner.detector_scheduler.clone(),
        )
    }

    /// Releases all the latches held by a command.
    fn release_lock(&self, lock: &Lock, cid: u64) {
        let wakeup_list = self.inner.latches.release(lock, cid);
        for wcid in wakeup_list {
            self.try_to_wake_up(wcid);
        }
    }

    fn schedule_command(&self, cmd: Command, callback: StorageCb) {
        let cid = self.inner.gen_id();
        debug!("received new command"; "cid" => cid, "cmd" => ?cmd);

        let tag = cmd.tag();
        let priority_tag = cmd.priority_tag();
        let task = Task::new(cid, cmd);
        // TODO: enqueue_task should return an reference of the tctx.
        self.inner.enqueue_task(task, callback);
        self.try_to_wake_up(cid);
        SCHED_STAGE_COUNTER_VEC.get(tag).new.inc();
        SCHED_COMMANDS_PRI_COUNTER_VEC_STATIC
            .get(priority_tag)
            .inc();
    }

    /// Tries to acquire all the necessary latches. If all the necessary latches are acquired,
    /// the method initiates a get snapshot operation for furthur processing.
    fn try_to_wake_up(&self, cid: u64) {
        if self.inner.acquire_lock(cid) {
            self.get_snapshot(cid);
        }
    }

    fn on_receive_new_cmd(&self, cmd: Command, callback: StorageCb) {
        // write flow control
        if cmd.need_flow_control() && self.inner.too_busy() {
            SCHED_TOO_BUSY_COUNTER_VEC.get(cmd.tag()).inc();
            execute_callback(
                callback,
                ProcessResult::Failed {
                    err: StorageError::SchedTooBusy,
                },
            );
            return;
        }
        self.schedule_command(cmd, callback);
    }

    /// Initiates an async operation to get a snapshot from the storage engine, then posts a
    /// `SnapshotFinished` message back to the event loop when it finishes.
    fn get_snapshot(&self, cid: u64) {
        let task = self.inner.dequeue_task(cid);
        let tag = task.tag;
        let ctx = task.context().clone();
        let executor = self.fetch_executor(task.priority(), task.cmd().is_sys_cmd());

        let cb = Box::new(move |(cb_ctx, snapshot)| {
            executor.execute(cb_ctx, snapshot, task);
        });

        let f = |engine: &E| {
            if let Err(e) = engine.async_snapshot(&ctx, cb) {
                SCHED_STAGE_COUNTER_VEC.get(tag).async_snapshot_err.inc();

                error!("engine async_snapshot failed"; "err" => ?e);
                self.finish_with_err(cid, e.into());
            } else {
                SCHED_STAGE_COUNTER_VEC.get(tag).snapshot.inc();
            }
        };

        if let Some(engine) = self.engine.as_ref() {
            f(engine)
        } else {
            // The program is currently in scheduler worker threads.
            with_tls_engine(f)
        }
    }

    /// Calls the callback with an error.
    fn finish_with_err(&self, cid: u64, err: Error) {
        debug!("write command finished with error"; "cid" => cid);
        let tctx = self.inner.dequeue_task_context(cid);

        SCHED_STAGE_COUNTER_VEC.get(tctx.tag).error.inc();

        let pr = ProcessResult::Failed {
            err: StorageError::from(err),
        };
        execute_callback(tctx.cb, pr);

        self.release_lock(&tctx.lock, cid);
    }

    /// Event handler for the success of read.
    ///
    /// If a next command is present, continues to execute; otherwise, delivers the result to the
    /// callback.
    fn on_read_finished(&self, cid: u64, pr: ProcessResult, tag: CommandKind) {
        SCHED_STAGE_COUNTER_VEC.get(tag).read_finish.inc();

        debug!("read command finished"; "cid" => cid);
        let tctx = self.inner.dequeue_task_context(cid);
        if let ProcessResult::NextCommand { cmd } = pr {
            SCHED_STAGE_COUNTER_VEC.get(tag).next_cmd.inc();
            self.schedule_command(cmd, tctx.cb);
        } else {
            execute_callback(tctx.cb, pr);
        }

        self.release_lock(&tctx.lock, cid);
    }

    /// Event handler for the success of write.
    fn on_write_finished(
        &self,
        cid: u64,
        pr: ProcessResult,
        result: EngineResult<()>,
        tag: CommandKind,
    ) {
        SCHED_STAGE_COUNTER_VEC.get(tag).write_finish.inc();

        debug!("write command finished"; "cid" => cid);
        let tctx = self.inner.dequeue_task_context(cid);
        let pr = match result {
            Ok(()) => pr,
            Err(e) => ProcessResult::Failed {
                err: StorageError::from(e),
            },
        };
        if let ProcessResult::NextCommand { cmd } = pr {
            SCHED_STAGE_COUNTER_VEC.get(tag).next_cmd.inc();
            self.schedule_command(cmd, tctx.cb);
        } else {
            execute_callback(tctx.cb, pr);
        }

        self.release_lock(&tctx.lock, cid);
    }

    /// Event handler for the request of waiting for lock
    fn on_wait_for_lock(
        &self,
        cid: u64,
        start_ts: u64,
        pr: ProcessResult,
        lock: lock_manager::Lock,
        is_first_lock: bool,
    ) {
        debug!("command waits for lock released"; "cid" => cid);
        let tctx = self.inner.dequeue_task_context(cid);
        SCHED_STAGE_COUNTER_VEC.get(tctx.tag).lock_wait.inc();
        self.inner.waiter_mgr_scheduler.as_ref().unwrap().wait_for(
            start_ts,
            tctx.cb,
            pr,
            lock.clone(),
            is_first_lock,
        );
        // Set `WAIT_TABLE_IS_EMPTY` here to prevent there is an on-the-fly WaitFor msg
        // but the waiter_mgr haven't processed it, subsequent WakeUp msgs may be lost.
        //
        // But it's still possible that the waiter_mgr removes some waiters and set
        // `WAIT_TABLE_IS_EMPTY` to true just after we set it to false here.
        store_wait_table_is_empty(false);
        self.release_lock(&tctx.lock, cid);
    }
}

impl<E: Engine> MsgScheduler for Scheduler<E> {
    fn on_msg(&self, task: Msg) {
        match task {
            Msg::ReadFinished { cid, tag, pr } => self.on_read_finished(cid, pr, tag),
            Msg::WriteFinished {
                cid,
                tag,
                pr,
                result,
            } => self.on_write_finished(cid, pr, result, tag),
            Msg::FinishedWithErr { cid, err, .. } => self.finish_with_err(cid, err),
            Msg::WaitForLock {
                cid,
                start_ts,
                pr,
                lock,
                is_first_lock,
            } => self.on_wait_for_lock(cid, start_ts, pr, lock, is_first_lock),
            _ => unreachable!(),
        }
    }
}

fn gen_command_lock(latches: &Latches, cmd: &Command) -> Lock {
    match *cmd {
        Command::Prewrite { ref mutations, .. } => {
            let keys: Vec<&Key> = mutations.iter().map(|x| x.key()).collect();
            latches.gen_lock(&keys)
        }
        Command::ResolveLock { ref key_locks, .. } => {
            let keys: Vec<&Key> = key_locks.iter().map(|x| &x.0).collect();
            latches.gen_lock(&keys)
        }
        Command::AcquirePessimisticLock { ref keys, .. } => {
            let keys: Vec<&Key> = keys.iter().map(|x| &x.0).collect();
            latches.gen_lock(&keys)
        }
        Command::ResolveLockLite {
            ref resolve_keys, ..
        } => latches.gen_lock(resolve_keys),
        Command::Commit { ref keys, .. }
        | Command::Rollback { ref keys, .. }
        | Command::PessimisticRollback { ref keys, .. } => latches.gen_lock(keys),
        Command::Cleanup { ref key, .. } => latches.gen_lock(&[key]),
        Command::Pause { ref keys, .. } => latches.gen_lock(keys),
        _ => Lock::new(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mvcc;
    use crate::storage::txn::latch::*;
    use crate::storage::{Command, Key, Mutation, Options};
    use kvproto::kvrpcpb::Context;
    use tikv_util::collections::HashMap;

    #[test]
    fn test_command_latches() {
        let mut temp_map = HashMap::default();
        temp_map.insert(10, 20);
        let readonly_cmds = vec![
            Command::ScanLock {
                ctx: Context::new(),
                max_ts: 5,
                start_key: None,
                limit: 0,
            },
            Command::ResolveLock {
                ctx: Context::new(),
                txn_status: temp_map.clone(),
                scan_key: None,
                key_locks: vec![],
            },
            Command::MvccByKey {
                ctx: Context::new(),
                key: Key::from_raw(b"k"),
            },
            Command::MvccByStartTs {
                ctx: Context::new(),
                start_ts: 25,
            },
        ];
        let write_cmds = vec![
            Command::Prewrite {
                ctx: Context::new(),
                mutations: vec![Mutation::Put((Key::from_raw(b"k"), b"v".to_vec()))],
                primary: b"k".to_vec(),
                start_ts: 10,
                options: Options::default(),
            },
            Command::AcquirePessimisticLock {
                ctx: Context::new(),
                keys: vec![(Key::from_raw(b"k"), false)],
                primary: b"k".to_vec(),
                start_ts: 10,
                options: Options::default(),
            },
            Command::Commit {
                ctx: Context::new(),
                keys: vec![Key::from_raw(b"k")],
                lock_ts: 10,
                commit_ts: 20,
            },
            Command::Cleanup {
                ctx: Context::new(),
                key: Key::from_raw(b"k"),
                start_ts: 10,
            },
            Command::Rollback {
                ctx: Context::new(),
                keys: vec![Key::from_raw(b"k")],
                start_ts: 10,
            },
            Command::PessimisticRollback {
                ctx: Context::new(),
                keys: vec![Key::from_raw(b"k")],
                start_ts: 10,
                for_update_ts: 20,
            },
            Command::ResolveLock {
                ctx: Context::new(),
                txn_status: temp_map.clone(),
                scan_key: None,
                key_locks: vec![(
                    Key::from_raw(b"k"),
                    mvcc::Lock::new(mvcc::LockType::Put, b"k".to_vec(), 10, 20, None, 0, 0),
                )],
            },
            Command::ResolveLockLite {
                ctx: Context::new(),
                start_ts: 10,
                commit_ts: 0,
                resolve_keys: vec![Key::from_raw(b"k")],
            },
        ];

        let latches = Latches::new(1024);
        let write_locks: Vec<Lock> = write_cmds
            .into_iter()
            .enumerate()
            .map(|(id, cmd)| {
                let mut lock = gen_command_lock(&latches, &cmd);
                assert_eq!(latches.acquire(&mut lock, id as u64), id == 0);
                lock
            })
            .collect();

        for (id, cmd) in readonly_cmds.iter().enumerate() {
            let mut lock = gen_command_lock(&latches, cmd);
            assert!(latches.acquire(&mut lock, id as u64));
        }

        // acquire/release locks one by one.
        let max_id = write_locks.len() as u64 - 1;
        for (id, mut lock) in write_locks.into_iter().enumerate() {
            let id = id as u64;
            if id != 0 {
                assert!(latches.acquire(&mut lock, id));
            }
            let unlocked = latches.release(&lock, id);
            if id as u64 == max_id {
                assert!(unlocked.is_empty());
            } else {
                assert_eq!(unlocked, vec![id + 1]);
            }
        }
    }
}
