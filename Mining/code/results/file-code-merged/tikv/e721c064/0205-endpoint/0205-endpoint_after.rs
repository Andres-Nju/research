// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use std::cell::RefCell;
use std::f64::INFINITY;
use std::fmt;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam::atomic::AtomicCell;
use engine_rocks::RocksEngine;
use futures::future::Future;
use futures::sink::Sink;
use futures::stream::Stream;
use futures03::compat::Compat;
use futures03::compat::Compat01As03;
use futures03::future::FutureExt;
#[cfg(feature = "prost-codec")]
use kvproto::cdcpb::{
    event::Event as Event_oneof_event, ChangeDataRequest,
    DuplicateRequest as ErrorDuplicateRequest, Error as EventError, Event, ResolvedTs,
};
#[cfg(not(feature = "prost-codec"))]
use kvproto::cdcpb::{
    ChangeDataRequest, DuplicateRequest as ErrorDuplicateRequest, Error as EventError, Event,
    Event_oneof_event, ResolvedTs,
};
use kvproto::kvrpcpb::ExtraOp as TxnExtraOp;
use kvproto::metapb::Region;
use pd_client::PdClient;
use raftstore::coprocessor::CmdBatch;
use raftstore::router::RaftStoreRouter;
use raftstore::store::fsm::{ChangeCmd, ObserveID, StoreMeta};
use raftstore::store::msg::{Callback, ReadResponse, SignificantMsg};
use resolved_ts::Resolver;
use tikv::config::CdcConfig;
use tikv::storage::kv::Snapshot;
use tikv::storage::mvcc::{DeltaScanner, ScannerBuilder};
use tikv::storage::txn::TxnEntry;
use tikv::storage::txn::TxnEntryScanner;
use tikv::storage::Statistics;
use tikv_util::collections::HashMap;
use tikv_util::time::{Instant, Limiter};
use tikv_util::timer::{SteadyTimer, Timer};
use tikv_util::worker::{Runnable, RunnableWithTimer, ScheduleError, Scheduler};
use tokio_threadpool::{Builder, ThreadPool};
use txn_types::{Key, Lock, LockType, TimeStamp};

use crate::channel::SendError;
use crate::delegate::{Delegate, Downstream, DownstreamID, DownstreamState};
use crate::metrics::*;
use crate::service::{CdcEvent, Conn, ConnID, FeatureGate};
use crate::{CdcObserver, Error, Result};

pub enum Deregister {
    Downstream {
        region_id: u64,
        downstream_id: DownstreamID,
        conn_id: ConnID,
        err: Option<Error>,
    },
    Region {
        region_id: u64,
        observe_id: ObserveID,
        err: Error,
    },
    Conn(ConnID),
}

impl fmt::Display for Deregister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for Deregister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut de = f.debug_struct("Deregister");
        match self {
            Deregister::Downstream {
                ref region_id,
                ref downstream_id,
                ref conn_id,
                ref err,
            } => de
                .field("deregister", &"downstream")
                .field("region_id", region_id)
                .field("downstream_id", downstream_id)
                .field("conn_id", conn_id)
                .field("err", err)
                .finish(),
            Deregister::Region {
                ref region_id,
                ref observe_id,
                ref err,
            } => de
                .field("deregister", &"region")
                .field("region_id", region_id)
                .field("observe_id", observe_id)
                .field("err", err)
                .finish(),
            Deregister::Conn(ref conn_id) => de
                .field("deregister", &"conn")
                .field("conn_id", conn_id)
                .finish(),
        }
    }
}

type InitCallback = Box<dyn FnOnce() + Send>;
// Callback returns (old_value, statistics, is_cache_missed).
pub(crate) type OldValueCallback = Box<
    dyn FnMut(Key, TimeStamp, &mut OldValueStats) -> (Option<Vec<u8>>, Option<Statistics>) + Send,
>;

pub enum Validate {
    Region(u64, Box<dyn FnOnce(Option<&Delegate>) + Send>),
    OldValueCache(Box<dyn FnOnce(&OldValueStats) + Send>),
}

pub enum Task {
    Register {
        request: ChangeDataRequest,
        downstream: Downstream,
        conn_id: ConnID,
        version: semver::Version,
    },
    Deregister(Deregister),
    OpenConn {
        conn: Conn,
    },
    MultiBatch {
        multi: Vec<CmdBatch>,
        old_value_cb: OldValueCallback,
    },
    MinTS {
        regions: Vec<u64>,
        min_ts: TimeStamp,
    },
    ResolverReady {
        observe_id: ObserveID,
        region: Region,
        resolver: Resolver,
    },
    RegisterMinTsEvent,
    // The result of ChangeCmd should be returned from CDC Endpoint to ensure
    // the downstream switches to Normal after the previous commands was sunk.
    InitDownstream {
        downstream_id: DownstreamID,
        downstream_state: Arc<AtomicCell<DownstreamState>>,
        cb: InitCallback,
    },
    Validate(Validate),
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut de = f.debug_struct("CdcTask");
        match self {
            Task::Register {
                ref request,
                ref downstream,
                ref conn_id,
                ref version,
                ..
            } => de
                .field("type", &"register")
                .field("register request", request)
                .field("request", request)
                .field("id", &downstream.get_id())
                .field("conn_id", conn_id)
                .field("version", version)
                .finish(),
            Task::Deregister(deregister) => de
                .field("type", &"deregister")
                .field("deregister", deregister)
                .finish(),
            Task::OpenConn { ref conn } => de
                .field("type", &"open_conn")
                .field("conn_id", &conn.get_id())
                .finish(),
            Task::MultiBatch { multi, .. } => de
                .field("type", &"multibatch")
                .field("multibatch", &multi.len())
                .finish(),
            Task::MinTS { ref min_ts, .. } => {
                de.field("type", &"mit_ts").field("min_ts", min_ts).finish()
            }
            Task::ResolverReady {
                ref observe_id,
                ref region,
                ..
            } => de
                .field("type", &"resolver_ready")
                .field("observe_id", &observe_id)
                .field("region_id", &region.get_id())
                .finish(),
            Task::RegisterMinTsEvent => de.field("type", &"register_min_ts").finish(),
            Task::InitDownstream {
                ref downstream_id, ..
            } => de
                .field("type", &"init_downstream")
                .field("downstream", &downstream_id)
                .finish(),
            Task::Validate(validate) => match validate {
                Validate::Region(region_id, _) => de.field("region_id", &region_id).finish(),
                Validate::OldValueCache(_) => de.finish(),
            },
        }
    }
}

const METRICS_FLUSH_INTERVAL: u64 = 10_000; // 10s

#[derive(Default)]
pub struct OldValueStats {
    pub access_count: usize,
    pub miss_count: usize,
    pub miss_none_count: usize,
}

pub struct Endpoint<T> {
    capture_regions: HashMap<u64, Delegate>,
    connections: HashMap<ConnID, Conn>,
    scheduler: Scheduler<Task>,
    raft_router: T,
    observer: CdcObserver,

    pd_client: Arc<dyn PdClient>,
    timer: SteadyTimer,
    min_ts_interval: Duration,
    tso_worker: ThreadPool,
    store_meta: Arc<Mutex<StoreMeta>>,

    workers: ThreadPool,

    scan_speed_limter: Limiter,
    max_scan_batch_bytes: usize,
    max_scan_batch_size: usize,

    min_resolved_ts: TimeStamp,
    min_ts_region_id: u64,

    // stats
    resolved_region_count: usize,
    unresolved_region_count: usize,
    old_value_stats: OldValueStats,
}

impl<T: 'static + RaftStoreRouter> Endpoint<T> {
    pub fn new(
        cfg: &CdcConfig,
        pd_client: Arc<dyn PdClient>,
        scheduler: Scheduler<Task>,
        raft_router: T,
        observer: CdcObserver,
        store_meta: Arc<Mutex<StoreMeta>>,
    ) -> Endpoint<T> {
        let workers = Builder::new().name_prefix("cdcwkr").pool_size(4).build();
        let tso_worker = Builder::new().name_prefix("tso").pool_size(1).build();
        let speed_limter = Limiter::new(if cfg.incremental_scan_speed_limit.0 > 0 {
            cfg.incremental_scan_speed_limit.0 as f64
        } else {
            INFINITY
        });
        // For scan efficiency, the scan batch bytes should be around 1MB.
        // TODO: To avoid consume too much memory when there are many concurrent
        //       scan tasks (peak memory = 1MB * N tasks), we reduce the size
        //       to 16KB as a workaround.
        let max_scan_batch_bytes = 16 * 1024;
        // Assume 1KB per entry.
        let max_scan_batch_size = 1024;
        let ep = Endpoint {
            capture_regions: HashMap::default(),
            connections: HashMap::default(),
            scheduler,
            pd_client,
            tso_worker,
            timer: SteadyTimer::default(),
            scan_speed_limter: speed_limter,
            max_scan_batch_bytes,
            max_scan_batch_size,
            workers,
            raft_router,
            observer,
            store_meta,
            min_ts_interval: cfg.min_ts_interval.0,
            min_resolved_ts: TimeStamp::max(),
            min_ts_region_id: 0,
            resolved_region_count: 0,
            unresolved_region_count: 0,
            old_value_stats: OldValueStats::default(),
        };
        ep.register_min_ts_event();
        ep
    }

    pub fn new_timer(&self) -> Timer<()> {
        // Currently there is only one timeout for CDC.
        let cdc_timer_cap = 1;
        let mut timer = Timer::new(cdc_timer_cap);
        timer.add_task(Duration::from_millis(METRICS_FLUSH_INTERVAL), ());
        timer
    }

    pub fn set_min_ts_interval(&mut self, dur: Duration) {
        self.min_ts_interval = dur;
    }

    pub fn set_max_scan_batch_size(&mut self, max_scan_batch_size: usize) {
        self.max_scan_batch_size = max_scan_batch_size;
    }

    fn on_deregister(&mut self, deregister: Deregister) {
        info!("cdc deregister region"; "deregister" => ?deregister);
        fail_point!("cdc_before_handle_deregister", |_| {});
        match deregister {
            Deregister::Downstream {
                region_id,
                downstream_id,
                conn_id,
                err,
            } => {
                // The peer wants to deregister
                let mut is_last = false;
                if let Some(delegate) = self.capture_regions.get_mut(&region_id) {
                    is_last = delegate.unsubscribe(downstream_id, err);
                }
                if let Some(conn) = self.connections.get_mut(&conn_id) {
                    if let Some(id) = conn.downstream_id(region_id) {
                        if downstream_id == id {
                            conn.unsubscribe(region_id);
                        }
                    }
                }
                if is_last {
                    let delegate = self.capture_regions.remove(&region_id).unwrap();
                    if let Some(reader) = self.store_meta.lock().unwrap().readers.get(&region_id) {
                        reader.txn_extra_op.store(TxnExtraOp::Noop);
                    }
                    // Do not continue to observe the events of the region.
                    let oid = self.observer.unsubscribe_region(region_id, delegate.id);
                    assert!(
                        oid.is_some(),
                        "unsubscribe region {} failed, ObserveID {:?}",
                        region_id,
                        delegate.id
                    );
                }
            }
            Deregister::Region {
                region_id,
                observe_id,
                err,
            } => {
                // Something went wrong, deregister all downstreams of the region.

                // To avoid ABA problem, we must check the unique ObserveID.
                let need_remove = self
                    .capture_regions
                    .get(&region_id)
                    .map_or(false, |d| d.id == observe_id);
                if need_remove {
                    if let Some(mut delegate) = self.capture_regions.remove(&region_id) {
                        delegate.stop(err);
                    }
                    if let Some(reader) = self.store_meta.lock().unwrap().readers.get(&region_id) {
                        reader.txn_extra_op.store(TxnExtraOp::Noop);
                    }
                    self.connections
                        .iter_mut()
                        .for_each(|(_, conn)| conn.unsubscribe(region_id));
                }
                // Do not continue to observe the events of the region.
                let oid = self.observer.unsubscribe_region(region_id, observe_id);
                assert_eq!(
                    need_remove,
                    oid.is_some(),
                    "unsubscribe region {} failed, ObserveID {:?}",
                    region_id,
                    observe_id
                );
            }
            Deregister::Conn(conn_id) => {
                // The connection is closed, deregister all downstreams of the connection.
                if let Some(conn) = self.connections.remove(&conn_id) {
                    conn.take_downstreams()
                        .into_iter()
                        .for_each(|(region_id, downstream_id)| {
                            if let Some(delegate) = self.capture_regions.get_mut(&region_id) {
                                if delegate.unsubscribe(downstream_id, None) {
                                    let delegate = self.capture_regions.remove(&region_id).unwrap();
                                    // Do not continue to observe the events of the region.
                                    let oid =
                                        self.observer.unsubscribe_region(region_id, delegate.id);
                                    assert!(
                                        oid.is_some(),
                                        "unsubscribe region {} failed, ObserveID {:?}",
                                        region_id,
                                        delegate.id
                                    );
                                }
                            }
                        });
                }
            }
        }
    }

    pub fn on_register(
        &mut self,
        mut request: ChangeDataRequest,
        mut downstream: Downstream,
        conn_id: ConnID,
        version: semver::Version,
    ) {
        let region_id = request.region_id;
        let downstream_id = downstream.get_id();
        let conn = match self.connections.get_mut(&conn_id) {
            Some(conn) => conn,
            None => {
                error!("cdc register for a nonexistent connection";
                    "region_id" => region_id, "conn_id" => ?conn_id);
                return;
            }
        };
        downstream.set_sink(conn.get_sink().clone());

        // TODO: Add a new task to close incompatible features.
        if let Some(e) = conn.check_version_and_set_feature(version) {
            // The downstream has not registered yet, send error right away.
            let mut err_event = EventError::default();
            err_event.set_compatibility(e);
            let _ = downstream.sink_error_event(region_id, err_event);
            return;
        }
        if !conn.subscribe(region_id, downstream_id) {
            let mut err_event = EventError::default();
            let mut err = ErrorDuplicateRequest::default();
            err.set_region_id(region_id);
            err_event.set_duplicate_request(err);
            let _ = downstream.sink_error_event(region_id, err_event);
            error!("cdc duplicate register";
                "region_id" => region_id,
                "conn_id" => ?conn_id,
                "req_id" => request.get_request_id(),
                "downstream_id" => ?downstream_id);
            return;
        }

        info!("cdc register region";
            "region_id" => region_id,
            "conn_id" => ?conn.get_id(),
            "req_id" => request.get_request_id(),
            "downstream_id" => ?downstream_id);
        let mut is_new_delegate = false;
        let delegate = self.capture_regions.entry(region_id).or_insert_with(|| {
            let d = Delegate::new(region_id);
            is_new_delegate = true;
            d
        });

        let downstream_state = downstream.get_state();
        let checkpoint_ts = request.checkpoint_ts;
        let sched = self.scheduler.clone();

        if !delegate.subscribe(downstream) {
            conn.unsubscribe(request.get_region_id());
            if is_new_delegate {
                self.capture_regions.remove(&request.get_region_id());
            }
            return;
        }
        let change_cmd = if is_new_delegate {
            // The region has never been registered.
            // Subscribe the change events of the region.
            let old_id = self.observer.subscribe_region(region_id, delegate.id);
            assert!(
                old_id.is_none(),
                "region {} must not be observed twice, old ObserveID {:?}, new ObserveID {:?}",
                region_id,
                old_id,
                delegate.id
            );

            ChangeCmd::RegisterObserver {
                observe_id: delegate.id,
                region_id,
                enabled: delegate.enabled(),
            }
        } else {
            ChangeCmd::Snapshot {
                observe_id: delegate.id,
                region_id,
            }
        };
        let txn_extra_op = request.get_extra_op();
        if txn_extra_op != TxnExtraOp::Noop {
            delegate.txn_extra_op = request.get_extra_op();
            if let Some(reader) = self.store_meta.lock().unwrap().readers.get(&region_id) {
                reader.txn_extra_op.store(txn_extra_op);
            }
        }
        let observe_id = delegate.id;
        let mut init = Initializer {
            sched,
            region_id,
            conn_id,
            downstream_id,
            sink: conn.get_sink().clone(),
            request_id: request.get_request_id(),
            downstream_state: downstream_state.clone(),
            txn_extra_op: delegate.txn_extra_op,
            speed_limter: self.scan_speed_limter.clone(),
            max_scan_batch_bytes: self.max_scan_batch_bytes,
            max_scan_batch_size: self.max_scan_batch_size,
            observe_id,
            checkpoint_ts: checkpoint_ts.into(),
            build_resolver: is_new_delegate,
        };

        let (cb, fut) = tikv_util::future::paired_std_future_callback();
        let scheduler = self.scheduler.clone();
        let deregister_downstream = move |err| {
            warn!("cdc send capture change cmd failed"; "region_id" => region_id, "error" => ?err);
            let deregister = if is_new_delegate {
                // Deregister region if it's the first scan task, because the
                // task also build resolver.
                Deregister::Region {
                    region_id,
                    observe_id,
                    err,
                }
            } else {
                Deregister::Downstream {
                    region_id,
                    downstream_id,
                    conn_id,
                    err: Some(err),
                }
            };
            if let Err(e) = scheduler.schedule(Task::Deregister(deregister)) {
                error!("cdc schedule cdc task failed"; "error" => ?e);
            }
        };
        let scheduler = self.scheduler.clone();
        if let Err(e) = self.raft_router.significant_send(
            region_id,
            SignificantMsg::CaptureChange {
                cmd: change_cmd,
                region_epoch: request.take_region_epoch(),
                callback: Callback::Read(Box::new(move |resp| {
                    if let Err(e) = scheduler.schedule(Task::InitDownstream {
                        downstream_id,
                        downstream_state,
                        cb: Box::new(move || {
                            cb(resp);
                        }),
                    }) {
                        error!("cdc schedule cdc task failed"; "error" => ?e);
                    }
                })),
            },
        ) {
            deregister_downstream(Error::Request(e.into()));
            return;
        }
        self.workers.spawn(Compat::new(
            async move {
                match fut.await {
                    Ok(resp) => init.on_change_cmd(resp).await,
                    Err(e) => deregister_downstream(Error::Other(box_err!(e))),
                };
            }
            .unit_error()
            .boxed(),
        ));
    }

    pub fn on_multi_batch(&mut self, multi: Vec<CmdBatch>, old_value_cb: OldValueCallback) {
        fail_point!("cdc_before_handle_multi_batch", |_| {});
        let old_value_cb = Rc::new(RefCell::new(old_value_cb));
        for batch in multi {
            let region_id = batch.region_id;
            let mut deregister = None;
            if let Some(delegate) = self.capture_regions.get_mut(&region_id) {
                if delegate.has_failed() {
                    // Skip the batch if the delegate has failed.
                    continue;
                }
                if let Err(e) =
                    delegate.on_batch(batch, old_value_cb.clone(), &mut self.old_value_stats)
                {
                    assert!(delegate.has_failed());
                    // Delegate has error, deregister the corresponding region.
                    deregister = Some(Deregister::Region {
                        region_id,
                        observe_id: delegate.id,
                        err: e,
                    });
                }
            }
            if let Some(deregister) = deregister {
                self.on_deregister(deregister);
            }
        }
    }

    fn on_region_ready(&mut self, observe_id: ObserveID, resolver: Resolver, region: Region) {
        let region_id = region.get_id();
        if let Some(delegate) = self.capture_regions.get_mut(&region_id) {
            if delegate.id == observe_id {
                for downstream in delegate.on_region_ready(resolver, region) {
                    let conn_id = downstream.get_conn_id();
                    if !delegate.subscribe(downstream) {
                        let conn = self.connections.get_mut(&conn_id).unwrap();
                        conn.unsubscribe(region_id);
                    }
                }
            } else {
                debug!("cdc stale region ready";
                    "region_id" => region.get_id(),
                    "observe_id" => ?observe_id,
                    "current_id" => ?delegate.id);
            }
        } else {
            debug!("cdc region not found on region ready (finish building resolver)";
                "region_id" => region.get_id());
        }
    }

    fn on_min_ts(&mut self, regions: Vec<u64>, min_ts: TimeStamp) {
        let total_region_count = regions.len();
        let mut resolved_regions = Vec::with_capacity(regions.len());
        self.min_resolved_ts = TimeStamp::max();
        for region_id in regions {
            if let Some(delegate) = self.capture_regions.get_mut(&region_id) {
                if let Some(resolved_ts) = delegate.on_min_ts(min_ts) {
                    if resolved_ts < self.min_resolved_ts {
                        self.min_resolved_ts = resolved_ts;
                        self.min_ts_region_id = region_id;
                    }
                    resolved_regions.push(region_id);
                }
            }
        }
        self.resolved_region_count = resolved_regions.len();
        self.unresolved_region_count = total_region_count - self.resolved_region_count;
        self.broadcast_resolved_ts(resolved_regions);
    }

    fn broadcast_resolved_ts(&self, regions: Vec<u64>) {
        let mut resolved_ts = ResolvedTs::default();
        resolved_ts.regions = regions;
        resolved_ts.ts = self.min_resolved_ts.into_inner();

        let send_cdc_event = |conn: &Conn, event| {
            // No need force send, as resolved ts messages is sent regularly.
            // And errors can be ignored.
            match conn.get_sink().unbounded_send(event) {
                Ok(_) => (),
                Err(SendError::Disconnected) => {
                    debug!("cdc send event failed, disconnected";
                        "conn_id" => ?conn.get_id(), "downstream" => ?conn.get_peer());
                }
                Err(SendError::Full) | Err(SendError::Congested) => {
                    info!("cdc send event failed, full";
                        "conn_id" => ?conn.get_id(), "downstream" => ?conn.get_peer());
                }
            }
        };
        for conn in self.connections.values() {
            let features = if let Some(features) = conn.get_feature() {
                features
            } else {
                // None means there is no downsteam registered yet.
                continue;
            };

            if features.contains(FeatureGate::BATCH_RESOLVED_TS) {
                send_cdc_event(conn, CdcEvent::ResolvedTs(resolved_ts.clone()));
            } else {
                // Fallback to previous non-batch resolved ts event.
                for region_id in &resolved_ts.regions {
                    self.broadcast_resolved_ts_compact(*region_id, resolved_ts.ts, conn);
                }
            }
        }
    }

    fn broadcast_resolved_ts_compact(&self, region_id: u64, resolved_ts: u64, conn: &Conn) {
        let downstream_id = match conn.downstream_id(region_id) {
            Some(downstream_id) => downstream_id,
            // No such region registers in the connection.
            None => {
                debug!("cdc send resolved ts failed, no region downstream id found";
                    "region_id" => region_id);
                return;
            }
        };
        let delegate = match self.capture_regions.get(&region_id) {
            Some(delegate) => delegate,
            // No such region registers in the endpoint.
            None => {
                info!("cdc send resolved ts failed, no region delegate found";
                    "region_id" => region_id, "downstream_id" => ?downstream_id);
                return;
            }
        };
        let downstream = match delegate.downstream(downstream_id) {
            Some(downstream) => downstream,
            // No such downstream registers in the delegate.
            None => {
                info!("cdc send resolved ts failed, no region downstream found";
                    "region_id" => region_id, "downstream_id" => ?downstream_id);
                return;
            }
        };
        let mut resolved_ts_event = Event::default();
        resolved_ts_event.region_id = region_id;
        resolved_ts_event.event = Some(Event_oneof_event::ResolvedTs(resolved_ts));
        // No need force send, as resolved ts messages is sent regularly.
        // And errors can be ignored.
        let _ = downstream.sink_event(resolved_ts_event);
    }

    fn register_min_ts_event(&self) {
        let timeout = self.timer.delay(self.min_ts_interval);
        let pd_client = self.pd_client.clone();
        let scheduler = self.scheduler.clone();
        let raft_router = self.raft_router.clone();
        let regions: Vec<(u64, ObserveID)> = self
            .capture_regions
            .iter()
            .map(|(region_id, delegate)| (*region_id, delegate.id))
            .collect();
        let fut = timeout
            .map_err(|_| unreachable!())
            .then(move |_| pd_client.get_tso())
            .then(move |tso: pd_client::Result<TimeStamp>| {
                // Ignore get tso errors since we will retry every `min_ts_interval`.
                let min_ts = tso.unwrap_or_default();
                // TODO: send a message to raftstore would consume too much cpu time,
                // try to handle it outside raftstore.
                let regions: Vec<_> = regions
                    .iter()
                    .copied()
                    .map(|(region_id, observe_id)| {
                        let scheduler_clone = scheduler.clone();
                        let raft_router_clone = raft_router.clone();
                        futures::future::lazy(move || {
                            let (tx, rx) = futures::sync::mpsc::channel(5);
                            let tx_clone = tx.clone();
                            if let Err(e) = raft_router_clone.significant_send(
                                region_id,
                                SignificantMsg::LeaderCallback(Callback::Read(Box::new(
                                    move |resp| {
                                        let resp = if resp.response.get_header().has_error() {
                                            None
                                        } else {
                                            Some(region_id)
                                        };
                                        if let Err(err) =  tx_clone.send(resp).wait() {
                                            error!("cdc send tso response failed"; "region_id" => region_id, "err" => ?err);
                                        }
                                    },
                                ))),
                            ) {
                                warn!("cdc send LeaderCallback failed"; "err" => ?e, "min_ts" => min_ts);
                                let deregister = Deregister::Region {
                                    observe_id,
                                    region_id,
                                    err: Error::Request(e.into()),
                                };
                                if let Err(e) = scheduler_clone.schedule(Task::Deregister(deregister)) {
                                    error!("schedule cdc task failed"; "error" => ?e);
                                }
                                if let Err(err) =  tx.send(None).wait() {
                                    error!("cdc send tso response failed"; "region_id" => region_id, "err" => ?err);
                                }
                            }
                            rx.into_future()
                        })
                    })
                    .collect();
                    match scheduler.schedule(Task::RegisterMinTsEvent) {
                        Ok(_) | Err(ScheduleError::Stopped(_)) => (),
                        // Must schedule `RegisterMinTsEvent` event otherwise resolved ts can not
                        // advance normally.
                        Err(err) => panic!("failed to regiester min ts event, error: {:?}", err),
                    }
                    futures::future::join_all(regions).and_then(move |resps| {
                        let regions = resps.into_iter().filter_map(|(resp, _)| resp.unwrap_or(None)).collect::<Vec<u64>>();
                        if !regions.is_empty() {
                            match scheduler.schedule(Task::MinTS { regions, min_ts }) {
                                Ok(_) | Err(ScheduleError::Stopped(_)) => (),
                                Err(err) => panic!("failed to schedule min ts event, error: {:?}", err),
                            }
                        }
                        Ok(())
                    }).map_err(|_| ())
            },
        );
        self.tso_worker.spawn(fut);
    }

    fn on_open_conn(&mut self, conn: Conn) {
        self.connections.insert(conn.get_id(), conn);
    }
}

struct Initializer {
    sched: Scheduler<Task>,
    sink: crate::channel::Sink,

    region_id: u64,
    observe_id: ObserveID,
    downstream_id: DownstreamID,
    downstream_state: Arc<AtomicCell<DownstreamState>>,
    conn_id: ConnID,
    request_id: u64,
    checkpoint_ts: TimeStamp,
    txn_extra_op: TxnExtraOp,

    speed_limter: Limiter,
    max_scan_batch_bytes: usize,
    max_scan_batch_size: usize,

    build_resolver: bool,
}

impl Initializer {
    async fn on_change_cmd(&mut self, mut resp: ReadResponse<RocksEngine>) {
        CDC_SCAN_TASKS.with_label_values(&["total"]).inc();
        if let Some(region_snapshot) = resp.snapshot {
            assert_eq!(self.region_id, region_snapshot.get_region().get_id());
            let region = region_snapshot.get_region().clone();
            // Require barrier before finishing incremental scan, because
            // CDC needs to make sure resovled ts events can only be sent after
            // incremental scan is finished.
            let require_barrier = true;
            self.async_incremental_scan(region_snapshot, region, require_barrier)
                .await;
            CDC_SCAN_TASKS.with_label_values(&["finish"]).inc();
        } else {
            CDC_SCAN_TASKS.with_label_values(&["abort"]).inc();
            assert!(
                resp.response.get_header().has_error(),
                "no snapshot and no error? {:?}",
                resp.response
            );
            let err = resp.response.take_header().take_error();
            let deregister = Deregister::Region {
                region_id: self.region_id,
                observe_id: self.observe_id,
                err: Error::Request(err),
            };
            if let Err(e) = self.sched.schedule(Task::Deregister(deregister)) {
                error!("cdc schedule cdc task failed"; "error" => ?e);
            }
        }
    }

    async fn async_incremental_scan<S: Snapshot + 'static>(
        &mut self,
        snap: S,
        region: Region,
        require_barrier: bool,
    ) {
        let downstream_id = self.downstream_id;
        let region_id = region.get_id();
        debug!("cdc async incremental scan";
            "region_id" => region_id,
            "downstream_id" => ?downstream_id,
            "observe_id" => ?self.observe_id);

        let mut resolver = if self.build_resolver {
            Some(Resolver::new(region_id))
        } else {
            None
        };

        fail_point!("cdc_incremental_scan_start");

        let start = Instant::now_coarse();
        // Time range: (checkpoint_ts, current]
        let current = TimeStamp::max();
        let mut scanner = ScannerBuilder::new(snap, current, false)
            .range(None, None)
            .build_delta_scanner(self.checkpoint_ts, self.txn_extra_op)
            .unwrap();
        let conn_id = self.conn_id;
        let mut done = false;
        while !done {
            if self.downstream_state.load() != DownstreamState::Normal {
                info!("cdc async incremental scan canceled";
                    "region_id" => region_id,
                    "downstream_id" => ?downstream_id,
                    "observe_id" => ?self.observe_id,
                    "conn_id" => ?conn_id);
                self.deregister_downstream(None);
                return;
            }
            let entries = match self.scan_batch(&mut scanner, resolver.as_mut()).await {
                Ok(res) => res,
                Err(e) => {
                    error!("cdc scan entries failed"; "error" => ?e, "region_id" => region_id);
                    self.deregister_downstream(Some(e));
                    return;
                }
            };
            // If the last element is None, it means scanning is finished.
            if let Some(None) = entries.last() {
                done = true;
            }
            debug!("cdc scan entries"; "len" => entries.len(), "region_id" => region_id);
            fail_point!("before_schedule_incremental_scan");
            if let Err(e) = self.sink_scan_events(entries, done, require_barrier).await {
                self.deregister_downstream(Some(e));
                return;
            }
        }

        let takes = start.elapsed();
        if let Some(resolver) = resolver {
            self.finish_building_resolver(resolver, region, takes);
        }

        CDC_SCAN_DURATION_HISTOGRAM.observe(takes.as_secs_f64());
    }

    async fn scan_batch<S: Snapshot>(
        &self,
        scanner: &mut DeltaScanner<S>,
        resolver: Option<&mut Resolver>,
    ) -> Result<Vec<Option<TxnEntry>>> {
        fail_point!("cdc_scan_batch_fail", |_| {
            Err(Error::Rocks("injected error".to_string()))
        });

        let mut entries = Vec::with_capacity(self.max_scan_batch_size);
        let mut total_bytes = 0;
        let mut total_size = 0;
        while total_bytes <= self.max_scan_batch_bytes && total_size < self.max_scan_batch_size {
            total_size += 1;
            match scanner.next_entry()? {
                Some(entry) => {
                    total_bytes += entry.size();
                    entries.push(Some(entry));
                }
                None => {
                    entries.push(None);
                    break;
                }
            }
        }
        if total_bytes > 0 {
            self.speed_limter.consume(total_bytes).await;
            CDC_SCAN_BYTES.inc_by(total_bytes as _);
        }

        if let Some(resolver) = resolver {
            // Track the locks.
            for entry in &entries {
                if let Some(TxnEntry::Prewrite { lock, .. }) = entry {
                    let (encoded_key, value) = lock;
                    let key = Key::from_encoded_slice(encoded_key).into_raw().unwrap();
                    let lock = Lock::parse(value)?;
                    match lock.lock_type {
                        LockType::Put | LockType::Delete => resolver.track_lock(lock.ts, key),
                        _ => (),
                    };
                }
            }
        }

        Ok(entries)
    }

    async fn sink_scan_events(
        &mut self,
        entries: Vec<Option<TxnEntry>>,
        done: bool,
        require_barrier: bool,
    ) -> Result<()> {
        let mut barrier = None;
        let mut events = Delegate::convert_to_grpc_events(self.region_id, self.request_id, entries);
        if done {
            let (cb, fut) = tikv_util::future::paired_future_callback();
            events.push(CdcEvent::Barrier(Some(cb)));
            barrier = Some(Compat01As03::new(fut));
        }
        if let Err(e) = self.sink.send_all(events).await {
            error!("cdc send scan event failed"; "req_id" => ?self.request_id);
            return Err(Error::Sink(e));
        }
        if require_barrier {
            if let Some(barrier) = barrier {
                // Make sure tikv sends out all scan events.
                let _ = barrier.await;
            }
        }

        Ok(())
    }

    fn finish_building_resolver(&self, mut resolver: Resolver, region: Region, takes: Duration) {
        let observe_id = self.observe_id;
        resolver.init();
        let rts = resolver.resolve(TimeStamp::zero());
        info!(
            "cdc resolver initialized and schedule resolver ready";
            "region_id" => region.get_id(),
            "conn_id" => ?self.conn_id,
            "downstream_id" => ?self.downstream_id,
            "resolved_ts" => rts,
            "lock_count" => resolver.locks().len(),
            "observe_id" => ?observe_id,
            "takes" => ?takes,
        );

        fail_point!("before_schedule_resolver_ready");
        if let Err(e) = self.sched.schedule(Task::ResolverReady {
            observe_id,
            resolver,
            region,
        }) {
            error!("cdc schedule task failed"; "error" => ?e);
        }
    }

    fn deregister_downstream(&self, err: Option<Error>) {
        // TODO: record in metrics.
        let deregister = if self.build_resolver {
            Deregister::Region {
                region_id: self.region_id,
                observe_id: self.observe_id,
                err: err.unwrap_or_else(|| Error::Other(box_err!("scan error"))), // TODO: convert rate_limiter error
            }
        } else {
            Deregister::Downstream {
                region_id: self.region_id,
                downstream_id: self.downstream_id,
                conn_id: self.conn_id,
                err, // TODO: convert rate_limiter error
            }
        };
        if let Err(e) = self.sched.schedule(Task::Deregister(deregister)) {
            error!("cdc schedule task failed"; "error" => ?e, "region_id" => self.region_id);
        }
    }
}

impl<T: 'static + RaftStoreRouter> Runnable<Task> for Endpoint<T> {
    fn run(&mut self, task: Task) {
        debug!("cdc run task"; "task" => %task);
        match task {
            Task::MinTS { regions, min_ts } => self.on_min_ts(regions, min_ts),
            Task::Register {
                request,
                downstream,
                conn_id,
                version,
            } => self.on_register(request, downstream, conn_id, version),
            Task::ResolverReady {
                observe_id,
                resolver,
                region,
            } => self.on_region_ready(observe_id, resolver, region),
            Task::Deregister(deregister) => self.on_deregister(deregister),
            Task::MultiBatch {
                multi,
                old_value_cb,
            } => self.on_multi_batch(multi, old_value_cb),
            Task::OpenConn { conn } => self.on_open_conn(conn),
            Task::RegisterMinTsEvent => self.register_min_ts_event(),
            Task::InitDownstream {
                downstream_id,
                downstream_state,
                cb,
            } => {
                debug!("cdc downstream was initialized"; "downstream_id" => ?downstream_id);
                let _ = downstream_state
                    .compare_exchange(DownstreamState::Uninitialized, DownstreamState::Normal);
                cb();
            }
            Task::Validate(validate) => match validate {
                Validate::Region(region_id, validate) => {
                    validate(self.capture_regions.get(&region_id));
                }
                Validate::OldValueCache(validate) => {
                    validate(&self.old_value_stats);
                }
            },
        }
    }
}

impl<T: 'static + RaftStoreRouter> RunnableWithTimer<Task, ()> for Endpoint<T> {
    fn on_timeout(&mut self, timer: &mut Timer<()>, _: ()) {
        CDC_CAPTURED_REGION_COUNT.set(self.capture_regions.len() as i64);
        CDC_REGION_RESOLVE_STATUS_GAUGE_VEC
            .with_label_values(&["unresolved"])
            .set(self.unresolved_region_count as _);
        CDC_REGION_RESOLVE_STATUS_GAUGE_VEC
            .with_label_values(&["resolved"])
            .set(self.resolved_region_count as _);
        if self.min_resolved_ts != TimeStamp::max() {
            CDC_MIN_RESOLVED_TS_REGION.set(self.min_ts_region_id as i64);
            CDC_MIN_RESOLVED_TS.set(self.min_resolved_ts.physical() as i64);
        }
        self.min_resolved_ts = TimeStamp::max();
        self.min_ts_region_id = 0;

        CDC_OLD_VALUE_CACHE_ACCESS.add(self.old_value_stats.access_count as i64);
        CDC_OLD_VALUE_CACHE_MISS.add(self.old_value_stats.miss_count as i64);
        CDC_OLD_VALUE_CACHE_MISS_NONE.add(self.old_value_stats.miss_none_count as i64);
        self.old_value_stats.access_count = 0;
        self.old_value_stats.miss_count = 0;
        self.old_value_stats.miss_none_count = 0;

        timer.add_task(Duration::from_millis(METRICS_FLUSH_INTERVAL), ());
    }
}

#[cfg(test)]
mod tests {
    use engine_traits::DATA_CFS;
    use futures03::executor::block_on;
    use kvproto::cdcpb::Header;
    #[cfg(feature = "prost-codec")]
    use kvproto::cdcpb::{event::Event as Event_oneof_event, Header};
    use kvproto::errorpb::Error as ErrorHeader;
    use kvproto::kvrpcpb::Context;
    use raftstore::errors::Error as RaftStoreError;
    use raftstore::store::msg::CasualMessage;
    use raftstore::store::ReadDelegate;
    use std::collections::BTreeMap;
    use std::fmt::Display;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
    use tempfile::TempDir;
    use test_raftstore::MockRaftStoreRouter;
    use test_raftstore::TestPdClient;
    use tikv::storage::kv::Engine;
    use tikv::storage::mvcc::tests::*;
    use tikv::storage::TestEngineBuilder;
    use tikv_util::collections::HashSet;
    use tikv_util::config::ReadableDuration;
    use tikv_util::worker::{dummy_scheduler, Builder as WorkerBuilder, Worker};
    use time::Timespec;

    use super::*;
    use crate::channel;

    struct ReceiverRunnable<T: Display + Send> {
        tx: Sender<T>,
    }

    impl<T: Display + Send> Runnable<T> for ReceiverRunnable<T> {
        fn run(&mut self, task: T) {
            self.tx.send(task).unwrap();
        }
    }

    fn new_receiver_worker<T: Display + Send + 'static>() -> (Worker<T>, Receiver<T>) {
        let (tx, rx) = channel();
        let runnable = ReceiverRunnable { tx };
        let mut worker = WorkerBuilder::new("test-receiver-worker").create();
        worker.start(runnable).unwrap();
        (worker, rx)
    }

    fn mock_initializer(
        speed_limit: usize,
        buffer: usize,
    ) -> (
        Worker<Task>,
        ThreadPool,
        Initializer,
        Receiver<Task>,
        crate::channel::Drain,
    ) {
        let (receiver_worker, rx) = new_receiver_worker();
        let (sink, drain) = crate::channel::canal(buffer);

        let pool = Builder::new()
            .name_prefix("test-initializer-worker")
            .pool_size(4)
            .build();
        let downstream_state = Arc::new(AtomicCell::new(DownstreamState::Normal));
        let initializer = Initializer {
            sched: receiver_worker.scheduler(),
            sink,

            region_id: 1,
            observe_id: ObserveID::new(),
            downstream_id: DownstreamID::new(),
            downstream_state,
            conn_id: ConnID::new(),
            request_id: 0,
            checkpoint_ts: 1.into(),
            speed_limter: Limiter::new(speed_limit as _),
            max_scan_batch_bytes: 1024 * 1024,
            max_scan_batch_size: 1024,
            txn_extra_op: TxnExtraOp::Noop,
            build_resolver: true,
        };

        (receiver_worker, pool, initializer, rx, drain)
    }

    #[test]
    fn test_initializer_build_resolver() {
        let temp = TempDir::new().unwrap();
        let engine = TestEngineBuilder::new()
            .path(temp.path())
            .cfs(DATA_CFS)
            .build()
            .unwrap();

        let mut expected_locks = BTreeMap::<TimeStamp, HashSet<Vec<u8>>>::new();

        let mut total_bytes = 0;
        // Pessimistic locks should not be tracked
        for i in 0..10 {
            let k = &[b'k', i];
            total_bytes += k.len();
            let ts = TimeStamp::new(i as _);
            must_acquire_pessimistic_lock(&engine, k, k, ts, ts);
        }

        for i in 10..100 {
            let (k, v) = (&[b'k', i], &[b'v', i]);
            total_bytes += k.len();
            total_bytes += v.len();
            let ts = TimeStamp::new(i as _);
            must_prewrite_put(&engine, k, v, k, ts);
            expected_locks.entry(ts).or_default().insert(k.to_vec());
        }

        let mut region = Region::default();
        let snap = engine.snapshot(&Context::default()).unwrap();

        // Buffer must be large enough to unblock async incremental scan.
        let buffer = 1000;
        let (mut worker, _pool, mut initializer, rx, _drain) =
            mock_initializer(total_bytes, buffer);
        region.id = initializer.region_id;
        let check_result = || loop {
            let task = rx.recv().unwrap();
            match task {
                Task::ResolverReady { resolver, .. } => {
                    assert_eq!(resolver.locks(), &expected_locks);
                    return;
                }
                t => panic!("unepxected task {} received", t),
            }
        };
        // To not block test by barrier.
        let require_barrier = false;
        block_on(initializer.async_incremental_scan(snap.clone(), region.clone(), require_barrier));
        check_result();

        initializer.max_scan_batch_bytes = total_bytes;
        block_on(initializer.async_incremental_scan(snap.clone(), region.clone(), require_barrier));
        check_result();

        initializer.max_scan_batch_bytes = total_bytes / 3;
        let start_1_3 = Instant::now();
        block_on(initializer.async_incremental_scan(snap.clone(), region.clone(), require_barrier));
        check_result();
        // 2s to allow certain inaccuracy.
        assert!(
            start_1_3.elapsed() >= Duration::new(2, 0),
            "{:?}",
            start_1_3.elapsed()
        );

        let start_1_6 = Instant::now();
        initializer.max_scan_batch_bytes = total_bytes / 6;
        block_on(initializer.async_incremental_scan(snap.clone(), region.clone(), require_barrier));
        check_result();
        // 4s to allow certain inaccuracy.
        assert!(
            start_1_6.elapsed() >= Duration::new(4, 0),
            "{:?}",
            start_1_6.elapsed()
        );

        initializer.build_resolver = false;
        block_on(initializer.async_incremental_scan(snap.clone(), region.clone(), require_barrier));

        loop {
            let task = rx.recv_timeout(Duration::from_secs(1));
            match task {
                Ok(t) => panic!("unepxected task {} received", t),
                Err(RecvTimeoutError::Timeout) => break,
                Err(e) => panic!("unexpected err {:?}", e),
            }
        }

        // Test cancellation.
        initializer.downstream_state.store(DownstreamState::Stopped);
        block_on(initializer.async_incremental_scan(snap, region, require_barrier));

        loop {
            let task = rx.recv_timeout(Duration::from_secs(1));
            match task {
                Ok(Task::Deregister(Deregister::Downstream { region_id, .. })) => {
                    assert_eq!(region_id, initializer.region_id);
                    break;
                }
                Ok(other) => panic!("unexpected task {:?}", other),
                Err(e) => panic!("unexpected err {:?}", e),
            }
        }

        worker.stop().unwrap().join().unwrap();
    }

    #[test]
    fn test_raftstore_is_busy() {
        let (task_sched, task_rx) = dummy_scheduler();
        let raft_router = MockRaftStoreRouter::new();
        let observer = CdcObserver::new(task_sched.clone());
        let pd_client = Arc::new(TestPdClient::new(0, true));
        let mut ep = Endpoint::new(
            &CdcConfig::default(),
            pd_client,
            task_sched,
            raft_router.clone(),
            observer,
            Arc::new(Mutex::new(StoreMeta::new(0))),
        );
        let (tx, _rx) = channel::canal(1);
        // Fill the channel.
        let _raft_rx = raft_router.add_region(1 /* region id */, 1 /* cap */);
        loop {
            if let Err(RaftStoreError::Transport(_)) =
                raft_router.casual_send(1, CasualMessage::ClearRegionSize)
            {
                break;
            }
        }
        // Make sure channel is full.
        raft_router
            .casual_send(1, CasualMessage::ClearRegionSize)
            .unwrap_err();

        let conn = Conn::new(tx, String::new());
        let conn_id = conn.get_id();
        ep.run(Task::OpenConn { conn });
        let mut req_header = Header::default();
        req_header.set_cluster_id(0);
        let mut req = ChangeDataRequest::default();
        req.set_region_id(1);
        let region_epoch = req.get_region_epoch().clone();
        let downstream = Downstream::new("".to_string(), region_epoch, 0, conn_id, true);
        ep.run(Task::Register {
            request: req,
            downstream,
            conn_id,
            version: semver::Version::new(0, 0, 0),
        });
        assert_eq!(ep.capture_regions.len(), 1);

        for _ in 0..5 {
            if let Ok(Some(Task::Deregister(Deregister::Downstream { err, .. }))) =
                task_rx.recv_timeout(Duration::from_secs(1))
            {
                if let Some(Error::Request(err)) = err {
                    assert!(!err.has_server_is_busy());
                }
            }
        }
    }

    #[test]
    fn test_register() {
        let (task_sched, task_rx) = dummy_scheduler();
        let raft_router = MockRaftStoreRouter::new();
        let _raft_rx = raft_router.add_region(1 /* region id */, 100 /* cap */);
        let observer = CdcObserver::new(task_sched.clone());
        let pd_client = Arc::new(TestPdClient::new(0, true));
        let mut ep = Endpoint::new(
            &CdcConfig {
                min_ts_interval: ReadableDuration(Duration::from_secs(60)),
                ..Default::default()
            },
            pd_client,
            task_sched,
            raft_router.clone(),
            observer,
            Arc::new(Mutex::new(StoreMeta::new(0))),
        );
        let (tx, rx) = channel::canal(1);
        let mut rx = rx.drain();

        let conn = Conn::new(tx, String::new());
        let conn_id = conn.get_id();
        ep.run(Task::OpenConn { conn });
        let mut req_header = Header::default();
        req_header.set_cluster_id(0);
        let mut req = ChangeDataRequest::default();
        req.set_region_id(1);
        let region_epoch = req.get_region_epoch().clone();
        let downstream = Downstream::new("".to_string(), region_epoch.clone(), 1, conn_id, true);
        ep.run(Task::Register {
            request: req.clone(),
            downstream,
            conn_id,
            version: semver::Version::new(4, 0, 8),
        });
        assert_eq!(ep.capture_regions.len(), 1);

        // duplicate request error.
        let downstream = Downstream::new("".to_string(), region_epoch.clone(), 2, conn_id, true);
        ep.run(Task::Register {
            request: req.clone(),
            downstream,
            conn_id,
            version: semver::Version::new(4, 0, 8),
        });
        let cdc_event = channel::recv_timeout(&mut rx, Duration::from_millis(500))
            .unwrap()
            .unwrap();
        if let CdcEvent::Event(mut e) = cdc_event.0 {
            assert_eq!(e.region_id, 1);
            assert_eq!(e.request_id, 2);
            let event = e.event.take().unwrap();
            match event {
                Event_oneof_event::Error(err) => {
                    assert!(err.has_duplicate_request());
                }
                other => panic!("unknown event {:?}", other),
            }
        } else {
            panic!("unknown cdc event {:?}", cdc_event);
        }
        assert_eq!(ep.capture_regions.len(), 1);

        // Compatibility error.
        let downstream = Downstream::new("".to_string(), region_epoch, 3, conn_id, true);
        ep.run(Task::Register {
            request: req,
            downstream,
            conn_id,
            version: semver::Version::new(0, 0, 0),
        });
        let cdc_event = channel::recv_timeout(&mut rx, Duration::from_millis(500))
            .unwrap()
            .unwrap();
        if let CdcEvent::Event(mut e) = cdc_event.0 {
            assert_eq!(e.region_id, 1);
            assert_eq!(e.request_id, 3);
            let event = e.event.take().unwrap();
            match event {
                Event_oneof_event::Error(err) => {
                    assert!(err.has_compatibility());
                }
                other => panic!("unknown event {:?}", other),
            }
        } else {
            panic!("unknown cdc event {:?}", cdc_event);
        }
        assert_eq!(ep.capture_regions.len(), 1);

        // The first scan task of a region is initiated in register, and when it
        // fails, it should send a deregister region task, otherwise the region
        // delegate does not have resolver.
        //
        // Test non-exist regoin in raft router.
        let mut req = ChangeDataRequest::default();
        req.set_region_id(100);
        let region_epoch = req.get_region_epoch().clone();
        let downstream = Downstream::new("".to_string(), region_epoch.clone(), 1, conn_id, true);
        ep.run(Task::Register {
            request: req.clone(),
            downstream,
            conn_id,
            version: semver::Version::new(4, 0, 8),
        });
        // Region 100 is inserted into capture_regions.
        assert_eq!(ep.capture_regions.len(), 2);
        let task = task_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        match task.unwrap() {
            Task::Deregister(Deregister::Region { region_id, err, .. }) => {
                assert_eq!(region_id, 100);
                assert!(matches!(err, Error::Request(_)), "{:?}", err);
            }
            other => panic!("unexpected task {:?}", other),
        }

        // Test errors on CaptureChange message.
        req.set_region_id(101);
        let raft_rx = raft_router.add_region(101 /* region id */, 100 /* cap */);
        let downstream = Downstream::new("".to_string(), region_epoch, 1, conn_id, true);
        ep.run(Task::Register {
            request: req,
            downstream,
            conn_id,
            version: semver::Version::new(4, 0, 8),
        });
        // Drop CaptureChange message, it should cause scan task failure.
        let _ = raft_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(ep.capture_regions.len(), 3);
        let task = task_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        match task.unwrap() {
            Task::Deregister(Deregister::Region { region_id, err, .. }) => {
                assert_eq!(region_id, 101);
                assert!(matches!(err, Error::Other(_)), "{:?}", err);
            }
            other => panic!("unexpected task {:?}", other),
        }
    }

    #[test]
    fn test_feature_gate() {
        let (task_sched, _task_rx) = dummy_scheduler();
        let raft_router = MockRaftStoreRouter::new();
        let _raft_rx = raft_router.add_region(1 /* region id */, 100 /* cap */);
        let observer = CdcObserver::new(task_sched.clone());
        let pd_client = Arc::new(TestPdClient::new(0, true));
        let mut ep = Endpoint::new(
            &CdcConfig {
                min_ts_interval: ReadableDuration(Duration::from_secs(60)),
                ..Default::default()
            },
            pd_client,
            task_sched,
            raft_router,
            observer,
            Arc::new(Mutex::new(StoreMeta::new(0))),
        );

        let (tx, rx) = channel::canal(1);
        let mut rx = rx.drain();
        let mut region = Region::default();
        region.set_id(1);
        let conn = Conn::new(tx, String::new());
        let conn_id = conn.get_id();
        ep.run(Task::OpenConn { conn });
        let mut req_header = Header::default();
        req_header.set_cluster_id(0);
        let mut req = ChangeDataRequest::default();
        req.set_region_id(1);
        let region_epoch = req.get_region_epoch().clone();
        let downstream = Downstream::new("".to_string(), region_epoch.clone(), 0, conn_id, true);
        ep.run(Task::Register {
            request: req.clone(),
            downstream,
            conn_id,
            version: semver::Version::new(4, 0, 8),
        });
        let mut resolver = Resolver::new(1);
        resolver.init();
        let observe_id = ep.capture_regions[&1].id;
        ep.on_region_ready(observe_id, resolver, region.clone());
        ep.run(Task::MinTS {
            regions: vec![1],
            min_ts: TimeStamp::from(1),
        });
        let cdc_event = channel::recv_timeout(&mut rx, Duration::from_millis(500))
            .unwrap()
            .unwrap();
        if let CdcEvent::ResolvedTs(r) = cdc_event.0 {
            assert_eq!(r.regions, vec![1]);
            assert_eq!(r.ts, 1);
        } else {
            panic!("unknown cdc event {:?}", cdc_event);
        }

        // Register region 2 to the conn.
        req.set_region_id(2);
        let downstream = Downstream::new("".to_string(), region_epoch.clone(), 0, conn_id, true);
        ep.run(Task::Register {
            request: req.clone(),
            downstream,
            conn_id,
            version: semver::Version::new(4, 0, 8),
        });
        let mut resolver = Resolver::new(2);
        resolver.init();
        region.set_id(2);
        let observe_id = ep.capture_regions[&2].id;
        ep.on_region_ready(observe_id, resolver, region);
        ep.run(Task::MinTS {
            regions: vec![1, 2],
            min_ts: TimeStamp::from(2),
        });
        let cdc_event = channel::recv_timeout(&mut rx, Duration::from_millis(500))
            .unwrap()
            .unwrap();
        if let CdcEvent::ResolvedTs(mut r) = cdc_event.0 {
            r.regions.as_mut_slice().sort();
            assert_eq!(r.regions, vec![1, 2]);
            assert_eq!(r.ts, 2);
        } else {
            panic!("unknown cdc event {:?}", cdc_event);
        }

        // Register region 3 to another conn which is not support batch resolved ts.
        let (tx, rx2) = channel::canal(1);
        let mut rx2 = rx2.drain();
        let mut region = Region::default();
        region.set_id(3);
        let conn = Conn::new(tx, String::new());
        let conn_id = conn.get_id();
        ep.run(Task::OpenConn { conn });
        req.set_region_id(3);
        let downstream = Downstream::new("".to_string(), region_epoch, 3, conn_id, true);
        ep.run(Task::Register {
            request: req,
            downstream,
            conn_id,
            version: semver::Version::new(4, 0, 5),
        });
        let mut resolver = Resolver::new(3);
        resolver.init();
        region.set_id(3);
        let observe_id = ep.capture_regions[&3].id;
        ep.on_region_ready(observe_id, resolver, region);
        ep.run(Task::MinTS {
            regions: vec![1, 2, 3],
            min_ts: TimeStamp::from(3),
        });
        let cdc_event = channel::recv_timeout(&mut rx, Duration::from_millis(500))
            .unwrap()
            .unwrap();
        if let CdcEvent::ResolvedTs(mut r) = cdc_event.0 {
            r.regions.as_mut_slice().sort();
            // Although region 3 is not register in the first conn, batch resolved ts
            // sends all region ids.
            assert_eq!(r.regions, vec![1, 2, 3]);
            assert_eq!(r.ts, 3);
        } else {
            panic!("unknown cdc event {:?}", cdc_event);
        }
        let cdc_event = channel::recv_timeout(&mut rx2, Duration::from_millis(500))
            .unwrap()
            .unwrap();
        if let CdcEvent::Event(mut e) = cdc_event.0 {
            assert_eq!(e.region_id, 3);
            assert_eq!(e.request_id, 3);
            let event = e.event.take().unwrap();
            match event {
                Event_oneof_event::ResolvedTs(ts) => {
                    assert_eq!(ts, 3);
                }
                other => panic!("unknown event {:?}", other),
            }
        } else {
            panic!("unknown cdc event {:?}", cdc_event);
        }
    }

    #[test]
    fn test_deregister() {
        let (task_sched, _task_rx) = dummy_scheduler();
        let raft_router = MockRaftStoreRouter::new();
        let _raft_rx = raft_router.add_region(1 /* region id */, 100 /* cap */);
        let mut region = Region::default();
        region.set_id(1);
        let store_meta = Arc::new(Mutex::new(StoreMeta::new(0)));
        let read_delegate = ReadDelegate {
            tag: String::new(),
            region: region.clone(),
            peer_id: 2,
            term: 1,
            applied_index_term: 1,
            leader_lease: None,
            last_valid_ts: RefCell::new(Timespec::new(0, 0)),
            invalid: Arc::new(AtomicBool::new(false)),
            txn_extra_op: Arc::new(AtomicCell::new(TxnExtraOp::default())),
        };
        store_meta.lock().unwrap().readers.insert(1, read_delegate);
        let observer = CdcObserver::new(task_sched.clone());
        let pd_client = Arc::new(TestPdClient::new(0, true));
        let mut ep = Endpoint::new(
            &CdcConfig::default(),
            pd_client,
            task_sched,
            raft_router,
            observer,
            store_meta,
        );
        let (tx, rx) = channel::canal(1);
        let mut rx = rx.drain();

        let conn = Conn::new(tx, String::new());
        let conn_id = conn.get_id();
        ep.run(Task::OpenConn { conn });
        let mut req_header = Header::default();
        req_header.set_cluster_id(0);
        let mut req = ChangeDataRequest::default();
        req.set_region_id(1);
        let region_epoch = req.get_region_epoch().clone();
        let downstream = Downstream::new("".to_string(), region_epoch.clone(), 0, conn_id, true);
        let downstream_id = downstream.get_id();
        ep.run(Task::Register {
            request: req.clone(),
            downstream,
            conn_id,
            version: semver::Version::new(0, 0, 0),
        });
        assert_eq!(ep.capture_regions.len(), 1);

        let mut err_header = ErrorHeader::default();
        err_header.set_not_leader(Default::default());
        let deregister = Deregister::Downstream {
            region_id: 1,
            downstream_id,
            conn_id,
            err: Some(Error::Request(err_header.clone())),
        };
        ep.run(Task::Deregister(deregister));
        loop {
            let cdc_event = channel::recv_timeout(&mut rx, Duration::from_millis(500))
                .unwrap()
                .unwrap();
            if let CdcEvent::Event(mut e) = cdc_event.0 {
                let event = e.event.take().unwrap();
                match event {
                    Event_oneof_event::Error(err) => {
                        assert!(err.has_not_leader());
                        break;
                    }
                    other => panic!("unknown event {:?}", other),
                }
            }
        }
        assert_eq!(ep.capture_regions.len(), 0);

        let downstream = Downstream::new("".to_string(), region_epoch.clone(), 0, conn_id, true);
        let new_downstream_id = downstream.get_id();
        ep.run(Task::Register {
            request: req.clone(),
            downstream,
            conn_id,
            version: semver::Version::new(0, 0, 0),
        });
        assert_eq!(ep.capture_regions.len(), 1);

        let deregister = Deregister::Downstream {
            region_id: 1,
            downstream_id,
            conn_id,
            err: Some(Error::Request(err_header.clone())),
        };
        ep.run(Task::Deregister(deregister));
        assert!(channel::recv_timeout(&mut rx, Duration::from_millis(200)).is_err());
        assert_eq!(ep.capture_regions.len(), 1);

        let deregister = Deregister::Downstream {
            region_id: 1,
            downstream_id: new_downstream_id,
            conn_id,
            err: Some(Error::Request(err_header.clone())),
        };
        ep.run(Task::Deregister(deregister));
        let cdc_event = channel::recv_timeout(&mut rx, Duration::from_millis(500))
            .unwrap()
            .unwrap();
        loop {
            if let CdcEvent::Event(mut e) = cdc_event.0 {
                let event = e.event.take().unwrap();
                match event {
                    Event_oneof_event::Error(err) => {
                        assert!(err.has_not_leader());
                        break;
                    }
                    other => panic!("unknown event {:?}", other),
                }
            }
        }
        assert_eq!(ep.capture_regions.len(), 0);

        // Stale deregister should be filtered.
        let downstream = Downstream::new("".to_string(), region_epoch, 0, conn_id, true);
        ep.run(Task::Register {
            request: req,
            downstream,
            conn_id,
            version: semver::Version::new(0, 0, 0),
        });
        assert_eq!(ep.capture_regions.len(), 1);
        let deregister = Deregister::Region {
            region_id: 1,
            // A stale ObserveID (different from the actual one).
            observe_id: ObserveID::new(),
            err: Error::Request(err_header),
        };
        ep.run(Task::Deregister(deregister));
        match channel::recv_timeout(&mut rx, Duration::from_millis(500)) {
            Err(_) => (),
            Ok(other) => panic!("unknown event {:?}", other),
        }
        assert_eq!(ep.capture_regions.len(), 1);
    }
}