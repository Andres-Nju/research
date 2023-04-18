    fn default() -> QueryCache<'tcx, M> {
        QueryCache {
            results: FxHashMap::default(),
            active: FxHashMap::default(),
            #[cfg(debug_assertions)]
            cache_hits: 0,
        }
    }
}

// If enabled, sends a message to the profile-queries thread.
macro_rules! profq_msg {
    ($tcx:expr, $msg:expr) => {
        if cfg!(debug_assertions) {
            if $tcx.sess.profile_queries() {
                profq_msg($tcx.sess, $msg)
            }
        }
    }
}

// If enabled, formats a key using its debug string, which can be
// expensive to compute (in terms of time).
macro_rules! profq_query_msg {
    ($query:expr, $tcx:expr, $key:expr) => {{
        let msg = if cfg!(debug_assertions) {
            if $tcx.sess.profile_queries_and_keys() {
                Some(format!("{:?}", $key))
            } else { None }
        } else { None };
        QueryMsg {
            query: $query,
            msg,
        }
    }}
}

/// A type representing the responsibility to execute the job in the `job` field.
/// This will poison the relevant query if dropped.
pub(super) struct JobOwner<'a, 'tcx, Q: QueryDescription<'tcx>> {
    cache: &'a Sharded<QueryCache<'tcx, Q>>,
    key: Q::Key,
    job: Lrc<QueryJob<'tcx>>,
}

impl<'a, 'tcx, Q: QueryDescription<'tcx>> JobOwner<'a, 'tcx, Q> {
    /// Either gets a `JobOwner` corresponding the query, allowing us to
    /// start executing the query, or returns with the result of the query.
    /// If the query is executing elsewhere, this will wait for it.
    /// If the query panicked, this will silently panic.
    ///
    /// This function is inlined because that results in a noticeable speed-up
    /// for some compile-time benchmarks.
    #[inline(always)]
    pub(super) fn try_get(tcx: TyCtxt<'tcx>, span: Span, key: &Q::Key) -> TryGetJob<'a, 'tcx, Q> {
        let cache = Q::query_cache(tcx);
        loop {
            let mut lock = cache.get_shard_by_value(key).lock();
            if let Some(value) = lock.results.get(key) {
                profq_msg!(tcx, ProfileQueriesMsg::CacheHit);
                tcx.sess.profiler(|p| p.record_query_hit(Q::NAME));
                let result = (value.value.clone(), value.index);
                #[cfg(debug_assertions)]
                {
                    lock.cache_hits += 1;
                }
                return TryGetJob::JobCompleted(result);
            }
            let job = match lock.active.entry((*key).clone()) {
                Entry::Occupied(entry) => {
                    match *entry.get() {
                        QueryResult::Started(ref job) => {
                            // For parallel queries, we'll block and wait until the query running
                            // in another thread has completed. Record how long we wait in the
                            // self-profiler.
                            #[cfg(parallel_compiler)]
                            tcx.sess.profiler(|p| p.query_blocked_start(Q::NAME));

                            job.clone()
                        },
                        QueryResult::Poisoned => FatalError.raise(),
                    }
                }
                Entry::Vacant(entry) => {
                    // No job entry for this query. Return a new one to be started later.
                    return tls::with_related_context(tcx, |icx| {
                        // Create the `parent` variable before `info`. This allows LLVM
                        // to elide the move of `info`
                        let parent = icx.query.clone();
                        let info = QueryInfo {
                            span,
                            query: Q::query(key.clone()),
                        };
                        let job = Lrc::new(QueryJob::new(info, parent));
                        let owner = JobOwner {
                            cache,
                            job: job.clone(),
                            key: (*key).clone(),
                        };
                        entry.insert(QueryResult::Started(job));
                        TryGetJob::NotYetStarted(owner)
                    })
                }
            };
            mem::drop(lock);

            // If we are single-threaded we know that we have cycle error,
            // so we just return the error.
            #[cfg(not(parallel_compiler))]
            return TryGetJob::Cycle(cold_path(|| {
                Q::handle_cycle_error(tcx, job.find_cycle_in_stack(tcx, span))
            }));

            // With parallel queries we might just have to wait on some other
            // thread.
            #[cfg(parallel_compiler)]
            {
                let result = job.r#await(tcx, span);
                tcx.sess.profiler(|p| p.query_blocked_end(Q::NAME));

                if let Err(cycle) = result {
                    return TryGetJob::Cycle(Q::handle_cycle_error(tcx, cycle));
                }
            }
        }
    }

    /// Completes the query by updating the query cache with the `result`,
    /// signals the waiter and forgets the JobOwner, so it won't poison the query
    #[inline(always)]
    pub(super) fn complete(self, result: &Q::Value, dep_node_index: DepNodeIndex) {
        // We can move out of `self` here because we `mem::forget` it below
        let key = unsafe { ptr::read(&self.key) };
        let job = unsafe { ptr::read(&self.job) };
        let cache = self.cache;

        // Forget ourself so our destructor won't poison the query
        mem::forget(self);

        let value = QueryValue::new(result.clone(), dep_node_index);
        {
            let mut lock = cache.get_shard_by_value(&key).lock();
            lock.active.remove(&key);
            lock.results.insert(key, value);
        }

        job.signal_complete();
    }
}

#[inline(always)]
fn with_diagnostics<F, R>(f: F) -> (R, ThinVec<Diagnostic>)
where
    F: FnOnce(Option<&Lock<ThinVec<Diagnostic>>>) -> R
{
    let diagnostics = Lock::new(ThinVec::new());
    let result = f(Some(&diagnostics));
    (result, diagnostics.into_inner())
}

impl<'a, 'tcx, Q: QueryDescription<'tcx>> Drop for JobOwner<'a, 'tcx, Q> {
    #[inline(never)]
    #[cold]
    fn drop(&mut self) {
        // Poison the query so jobs waiting on it panic.
        let shard = self.cache.get_shard_by_value(&self.key);
        shard.lock().active.insert(self.key.clone(), QueryResult::Poisoned);
        // Also signal the completion of the job, so waiters
        // will continue execution.
        self.job.signal_complete();
    }
}

#[derive(Clone)]
pub struct CycleError<'tcx> {
    /// The query and related span that uses the cycle.
    pub(super) usage: Option<(Span, Query<'tcx>)>,
    pub(super) cycle: Vec<QueryInfo<'tcx>>,
}

/// The result of `try_get_lock`.
pub(super) enum TryGetJob<'a, 'tcx, D: QueryDescription<'tcx>> {
    /// The query is not yet started. Contains a guard to the cache eventually used to start it.
    NotYetStarted(JobOwner<'a, 'tcx, D>),

    /// The query was already completed.
    /// Returns the result of the query and its dep-node index
    /// if it succeeded or a cycle error if it failed.
    JobCompleted((D::Value, DepNodeIndex)),

    /// Trying to execute the query resulted in a cycle.
    Cycle(D::Value),
}

impl<'tcx> TyCtxt<'tcx> {
    /// Executes a job by changing the `ImplicitCtxt` to point to the
    /// new query job while it executes. It returns the diagnostics
    /// captured during execution and the actual result.
    #[inline(always)]
    pub(super) fn start_query<F, R>(
        self,
        job: Lrc<QueryJob<'tcx>>,
        diagnostics: Option<&Lock<ThinVec<Diagnostic>>>,
        compute: F,
    ) -> R
    where
        F: FnOnce(TyCtxt<'tcx>) -> R,
    {
        // The `TyCtxt` stored in TLS has the same global interner lifetime
        // as `self`, so we use `with_related_context` to relate the 'tcx lifetimes
        // when accessing the `ImplicitCtxt`.
        tls::with_related_context(self, move |current_icx| {
            // Update the `ImplicitCtxt` to point to our new query job.
            let new_icx = tls::ImplicitCtxt {
                tcx: self.global_tcx(),
                query: Some(job),
                diagnostics,
                layout_depth: current_icx.layout_depth,
                task_deps: current_icx.task_deps,
            };

            // Use the `ImplicitCtxt` while we execute the query.
            tls::enter_context(&new_icx, |_| {
                compute(self.global_tcx())
            })
        })
    }

    #[inline(never)]
    #[cold]
    pub(super) fn report_cycle(
        self,
        CycleError { usage, cycle: stack }: CycleError<'tcx>,
    ) -> DiagnosticBuilder<'tcx> {
        assert!(!stack.is_empty());

        let fix_span = |span: Span, query: &Query<'tcx>| {
            self.sess.source_map().def_span(query.default_span(self, span))
        };

        // Disable naming impls with types in this path, since that
        // sometimes cycles itself, leading to extra cycle errors.
        // (And cycle errors around impls tend to occur during the
        // collect/coherence phases anyhow.)
        ty::print::with_forced_impl_filename_line(|| {
            let span = fix_span(stack[1 % stack.len()].span, &stack[0].query);
            let mut err = struct_span_err!(self.sess,
                                           span,
                                           E0391,
                                           "cycle detected when {}",
                                           stack[0].query.describe(self));

            for i in 1..stack.len() {
                let query = &stack[i].query;
                let span = fix_span(stack[(i + 1) % stack.len()].span, query);
                err.span_note(span, &format!("...which requires {}...", query.describe(self)));
            }

            err.note(&format!("...which again requires {}, completing the cycle",
                              stack[0].query.describe(self)));

            if let Some((span, query)) = usage {
                err.span_note(fix_span(span, &query),
                              &format!("cycle used when {}", query.describe(self)));
            }

            err
        })
    }

    pub fn try_print_query_stack(handler: &Handler) {
        eprintln!("query stack during panic:");

        tls::with_context_opt(|icx| {
            if let Some(icx) = icx {
                let mut current_query = icx.query.clone();
                let mut i = 0;

                while let Some(query) = current_query {
                    let mut diag = Diagnostic::new(Level::FailureNote,
                        &format!("#{} [{}] {}",
                                 i,
                                 query.info.query.name(),
                                 query.info.query.describe(icx.tcx)));
                    diag.span = icx.tcx.sess.source_map().def_span(query.info.span).into();
                    handler.force_print_diagnostic(diag);

                    current_query = query.parent.clone();
                    i += 1;
                }
            }
        });

        eprintln!("end of query stack");
    }

    #[inline(never)]
    pub(super) fn get_query<Q: QueryDescription<'tcx>>(self, span: Span, key: Q::Key) -> Q::Value {
        debug!("ty::query::get_query<{}>(key={:?}, span={:?})",
               Q::NAME.as_str(),
               key,
               span);

        profq_msg!(self,
            ProfileQueriesMsg::QueryBegin(
                span.data(),
                profq_query_msg!(Q::NAME.as_str(), self, key),
            )
        );

        let job = match JobOwner::try_get(self, span, &key) {
            TryGetJob::NotYetStarted(job) => job,
            TryGetJob::Cycle(result) => return result,
            TryGetJob::JobCompleted((v, index)) => {
                self.dep_graph.read_index(index);
                return v
            }
        };

        // Fast path for when incr. comp. is off. `to_dep_node` is
        // expensive for some `DepKind`s.
        if !self.dep_graph.is_fully_enabled() {
            let null_dep_node = DepNode::new_no_params(crate::dep_graph::DepKind::Null);
            return self.force_query_with_job::<Q>(key, job, null_dep_node).0;
        }

        if Q::ANON {
            profq_msg!(self, ProfileQueriesMsg::ProviderBegin);
            self.sess.profiler(|p| p.start_query(Q::NAME));

            let ((result, dep_node_index), diagnostics) = with_diagnostics(|diagnostics| {
                self.start_query(job.job.clone(), diagnostics, |tcx| {
                    tcx.dep_graph.with_anon_task(Q::dep_kind(), || {
                        Q::compute(tcx.global_tcx(), key)
                    })
                })
            });

            self.sess.profiler(|p| p.end_query(Q::NAME));
            profq_msg!(self, ProfileQueriesMsg::ProviderEnd);

            self.dep_graph.read_index(dep_node_index);

            if unlikely!(!diagnostics.is_empty()) {
                self.queries.on_disk_cache
                    .store_diagnostics_for_anon_node(dep_node_index, diagnostics);
            }

            job.complete(&result, dep_node_index);

            return result;
        }

        let dep_node = Q::to_dep_node(self, &key);

        if !Q::EVAL_ALWAYS {
            // The diagnostics for this query will be
            // promoted to the current session during
            // `try_mark_green()`, so we can ignore them here.
            let loaded = self.start_query(job.job.clone(), None, |tcx| {
                let marked = tcx.dep_graph.try_mark_green_and_read(tcx, &dep_node);
                marked.map(|(prev_dep_node_index, dep_node_index)| {
                    (tcx.load_from_disk_and_cache_in_memory::<Q>(
                        key.clone(),
                        prev_dep_node_index,
                        dep_node_index,
                        &dep_node
                    ), dep_node_index)
                })
            });
            if let Some((result, dep_node_index)) = loaded {
                job.complete(&result, dep_node_index);
                return result;
            }
        }

        let (result, dep_node_index) = self.force_query_with_job::<Q>(key, job, dep_node);
        self.dep_graph.read_index(dep_node_index);
        result
    }

    fn load_from_disk_and_cache_in_memory<Q: QueryDescription<'tcx>>(
        self,
        key: Q::Key,
        prev_dep_node_index: SerializedDepNodeIndex,
        dep_node_index: DepNodeIndex,
        dep_node: &DepNode,
    ) -> Q::Value {
        // Note this function can be called concurrently from the same query
        // We must ensure that this is handled correctly.

        debug_assert!(self.dep_graph.is_green(dep_node));

        // First we try to load the result from the on-disk cache.
        let result = if Q::cache_on_disk(self.global_tcx(), key.clone(), None) &&
                        self.sess.opts.debugging_opts.incremental_queries {
            self.sess.profiler(|p| p.incremental_load_result_start(Q::NAME));
            let result = Q::try_load_from_disk(self.global_tcx(), prev_dep_node_index);
            self.sess.profiler(|p| p.incremental_load_result_end(Q::NAME));

            // We always expect to find a cached result for things that
            // can be forced from `DepNode`.
            debug_assert!(!dep_node.kind.can_reconstruct_query_key() ||
                          result.is_some(),
                          "missing on-disk cache entry for {:?}",
                          dep_node);
            result
        } else {
            // Some things are never cached on disk.
            None
        };

        let result = if let Some(result) = result {
            profq_msg!(self, ProfileQueriesMsg::CacheHit);
            self.sess.profiler(|p| p.record_query_hit(Q::NAME));

            result
        } else {
            // We could not load a result from the on-disk cache, so
            // recompute.

            self.sess.profiler(|p| p.start_query(Q::NAME));

            // The dep-graph for this computation is already in-place.
            let result = self.dep_graph.with_ignore(|| {
                Q::compute(self, key)
            });

            self.sess.profiler(|p| p.end_query(Q::NAME));
            result
        };

        // If `-Zincremental-verify-ich` is specified, re-hash results from
        // the cache and make sure that they have the expected fingerprint.
        if unlikely!(self.sess.opts.debugging_opts.incremental_verify_ich) {
            self.incremental_verify_ich::<Q>(&result, dep_node, dep_node_index);
        }

        if unlikely!(self.sess.opts.debugging_opts.query_dep_graph) {
            self.dep_graph.mark_loaded_from_cache(dep_node_index, true);
        }

        result
    }

    #[inline(never)]
    #[cold]
    fn incremental_verify_ich<Q: QueryDescription<'tcx>>(
        self,
        result: &Q::Value,
        dep_node: &DepNode,
        dep_node_index: DepNodeIndex,
    ) {
        use crate::ich::Fingerprint;

        assert!(
            Some(self.dep_graph.fingerprint_of(dep_node_index)) ==
                self.dep_graph.prev_fingerprint_of(dep_node),
            "fingerprint for green query instance not loaded from cache: {:?}",
            dep_node,
        );

        debug!("BEGIN verify_ich({:?})", dep_node);
        let mut hcx = self.create_stable_hashing_context();

        let new_hash = Q::hash_result(&mut hcx, result).unwrap_or(Fingerprint::ZERO);
        debug!("END verify_ich({:?})", dep_node);

        let old_hash = self.dep_graph.fingerprint_of(dep_node_index);

        assert!(
            new_hash == old_hash,
            "found unstable fingerprints for {:?}",
            dep_node,
        );
    }

    #[inline(always)]
    fn force_query_with_job<Q: QueryDescription<'tcx>>(
        self,
        key: Q::Key,
        job: JobOwner<'_, 'tcx, Q>,
        dep_node: DepNode,
    ) -> (Q::Value, DepNodeIndex) {
        // If the following assertion triggers, it can have two reasons:
        // 1. Something is wrong with DepNode creation, either here or
        //    in `DepGraph::try_mark_green()`.
        // 2. Two distinct query keys get mapped to the same `DepNode`
        //    (see for example #48923).
        assert!(!self.dep_graph.dep_node_exists(&dep_node),
                "forcing query with already existing `DepNode`\n\
                 - query-key: {:?}\n\
                 - dep-node: {:?}",
                key, dep_node);

        profq_msg!(self, ProfileQueriesMsg::ProviderBegin);
        self.sess.profiler(|p| p.start_query(Q::NAME));

        let ((result, dep_node_index), diagnostics) = with_diagnostics(|diagnostics| {
            self.start_query(job.job.clone(), diagnostics, |tcx| {
                if Q::EVAL_ALWAYS {
                    tcx.dep_graph.with_eval_always_task(dep_node,
                                                        tcx,
                                                        key,
                                                        Q::compute,
                                                        Q::hash_result)
                } else {
                    tcx.dep_graph.with_task(dep_node,
                                            tcx,
                                            key,
                                            Q::compute,
                                            Q::hash_result)
                }
            })
        });

        self.sess.profiler(|p| p.end_query(Q::NAME));
        profq_msg!(self, ProfileQueriesMsg::ProviderEnd);

        if unlikely!(self.sess.opts.debugging_opts.query_dep_graph) {
            self.dep_graph.mark_loaded_from_cache(dep_node_index, false);
        }

        if unlikely!(!diagnostics.is_empty()) {
            if dep_node.kind != crate::dep_graph::DepKind::Null {
                self.queries.on_disk_cache
                    .store_diagnostics(dep_node_index, diagnostics);
            }
        }

        job.complete(&result, dep_node_index);

        (result, dep_node_index)
    }

    /// Ensure that either this query has all green inputs or been executed.
    /// Executing `query::ensure(D)` is considered a read of the dep-node `D`.
    ///
    /// This function is particularly useful when executing passes for their
    /// side-effects -- e.g., in order to report errors for erroneous programs.
    ///
    /// Note: The optimization is only available during incr. comp.
    pub(super) fn ensure_query<Q: QueryDescription<'tcx>>(self, key: Q::Key) -> () {
        if Q::EVAL_ALWAYS {
            let _ = self.get_query::<Q>(DUMMY_SP, key);
            return;
        }

        // Ensuring an anonymous query makes no sense
        assert!(!Q::ANON);

        let dep_node = Q::to_dep_node(self, &key);

        if self.dep_graph.try_mark_green_and_read(self, &dep_node).is_none() {
            // A None return from `try_mark_green_and_read` means that this is either
            // a new dep node or that the dep node has already been marked red.
            // Either way, we can't call `dep_graph.read()` as we don't have the
            // DepNodeIndex. We must invoke the query itself. The performance cost
            // this introduces should be negligible as we'll immediately hit the
            // in-memory cache, or another query down the line will.

            let _ = self.get_query::<Q>(DUMMY_SP, key);
        } else {
            profq_msg!(self, ProfileQueriesMsg::CacheHit);
            self.sess.profiler(|p| p.record_query_hit(Q::NAME));
        }
    }

    #[allow(dead_code)]
    fn force_query<Q: QueryDescription<'tcx>>(self, key: Q::Key, span: Span, dep_node: DepNode) {
        profq_msg!(
            self,
            ProfileQueriesMsg::QueryBegin(span.data(),
                                          profq_query_msg!(Q::NAME.as_str(), self, key))
        );

        // We may be concurrently trying both execute and force a query.
        // Ensure that only one of them runs the query.
        let job = match JobOwner::try_get(self, span, &key) {
            TryGetJob::NotYetStarted(job) => job,
            TryGetJob::Cycle(_) |
            TryGetJob::JobCompleted(_) => {
                return
            }
        };
        self.force_query_with_job::<Q>(key, job, dep_node);
    }
}

macro_rules! handle_cycle_error {
    ([][$tcx: expr, $error:expr]) => {{
        $tcx.report_cycle($error).emit();
        Value::from_cycle_error($tcx.global_tcx())
    }};
    ([fatal_cycle$(, $modifiers:ident)*][$tcx:expr, $error:expr]) => {{
        $tcx.report_cycle($error).emit();
        $tcx.sess.abort_if_errors();
        unreachable!()
    }};
    ([cycle_delay_bug$(, $modifiers:ident)*][$tcx:expr, $error:expr]) => {{
        $tcx.report_cycle($error).delay_as_bug();
        Value::from_cycle_error($tcx.global_tcx())
    }};
    ([$other:ident$(, $modifiers:ident)*][$($args:tt)*]) => {
        handle_cycle_error!([$($modifiers),*][$($args)*])
    };
}

macro_rules! is_anon {
    ([]) => {{
        false
    }};
    ([anon$(, $modifiers:ident)*]) => {{
        true
    }};
    ([$other:ident$(, $modifiers:ident)*]) => {
        is_anon!([$($modifiers),*])
    };
}

macro_rules! is_eval_always {
    ([]) => {{
        false
    }};
    ([eval_always$(, $modifiers:ident)*]) => {{
        true
    }};
    ([$other:ident$(, $modifiers:ident)*]) => {
        is_eval_always!([$($modifiers),*])
    };
}

macro_rules! hash_result {
    ([][$hcx:expr, $result:expr]) => {{
        dep_graph::hash_result($hcx, &$result)
    }};
    ([no_hash$(, $modifiers:ident)*][$hcx:expr, $result:expr]) => {{
        None
    }};
    ([$other:ident$(, $modifiers:ident)*][$($args:tt)*]) => {
        hash_result!([$($modifiers),*][$($args)*])
    };
}

macro_rules! define_queries {
    (<$tcx:tt> $($category:tt {
        $($(#[$attr:meta])* [$($modifiers:tt)*] fn $name:ident: $node:ident($K:ty) -> $V:ty,)*
    },)*) => {
        define_queries_inner! { <$tcx>
            $($( $(#[$attr])* category<$category> [$($modifiers)*] fn $name: $node($K) -> $V,)*)*
        }
    }
}

macro_rules! define_queries_inner {
    (<$tcx:tt>
     $($(#[$attr:meta])* category<$category:tt>
        [$($modifiers:tt)*] fn $name:ident: $node:ident($K:ty) -> $V:ty,)*) => {

        use std::mem;
        #[cfg(parallel_compiler)]
        use ty::query::job::QueryResult;
        use rustc_data_structures::sharded::Sharded;
        use crate::{
            rustc_data_structures::stable_hasher::HashStable,
            rustc_data_structures::stable_hasher::StableHasherResult,
            rustc_data_structures::stable_hasher::StableHasher,
            ich::StableHashingContext
        };
        use crate::util::profiling::ProfileCategory;

        define_queries_struct! {
            tcx: $tcx,
            input: ($(([$($modifiers)*] [$($attr)*] [$name]))*)
        }

        impl<$tcx> Queries<$tcx> {
            pub fn new(
                providers: IndexVec<CrateNum, Providers<$tcx>>,
                fallback_extern_providers: Providers<$tcx>,
                on_disk_cache: OnDiskCache<'tcx>,
            ) -> Self {
                Queries {
                    providers,
                    fallback_extern_providers: Box::new(fallback_extern_providers),
                    on_disk_cache,
                    $($name: Default::default()),*
                }
            }

            #[cfg(parallel_compiler)]
            pub fn collect_active_jobs(&self) -> Vec<Lrc<QueryJob<$tcx>>> {
                let mut jobs = Vec::new();

                // We use try_lock_shards here since we are only called from the
                // deadlock handler, and this shouldn't be locked.
                $(
                    let shards = self.$name.try_lock_shards().unwrap();
                    jobs.extend(shards.iter().flat_map(|shard| shard.active.values().filter_map(|v|
                        if let QueryResult::Started(ref job) = *v {
                            Some(job.clone())
                        } else {
                            None
                        }
                    )));
                )*

                jobs
            }

            pub fn print_stats(&self) {
                let mut queries = Vec::new();

                #[derive(Clone)]
                struct QueryStats {
                    name: &'static str,
                    cache_hits: usize,
                    key_size: usize,
                    key_type: &'static str,
                    value_size: usize,
                    value_type: &'static str,
                    entry_count: usize,
                }

                fn stats<'tcx, Q: QueryConfig<'tcx>>(
                    name: &'static str,
                    map: &Sharded<QueryCache<'tcx, Q>>,
                ) -> QueryStats {
                    let map = map.lock_shards();
                    QueryStats {
                        name,
                        #[cfg(debug_assertions)]
                        cache_hits: map.iter().map(|shard| shard.cache_hits).sum(),
                        #[cfg(not(debug_assertions))]
                        cache_hits: 0,
                        key_size: mem::size_of::<Q::Key>(),
                        key_type: type_name::<Q::Key>(),
                        value_size: mem::size_of::<Q::Value>(),
                        value_type: type_name::<Q::Value>(),
                        entry_count: map.iter().map(|shard| shard.results.len()).sum(),
                    }
                }

                $(
                    queries.push(stats::<queries::$name<'_>>(
                        stringify!($name),
                        &self.$name,
                    ));
                )*

                if cfg!(debug_assertions) {
                    let hits: usize = queries.iter().map(|s| s.cache_hits).sum();
                    let results: usize = queries.iter().map(|s| s.entry_count).sum();
                    println!("\nQuery cache hit rate: {}", hits as f64 / (hits + results) as f64);
                }

                let mut query_key_sizes = queries.clone();
                query_key_sizes.sort_by_key(|q| q.key_size);
                println!("\nLarge query keys:");
                for q in query_key_sizes.iter().rev()
                                        .filter(|q| q.key_size > 8) {
                    println!(
                        "   {} - {} x {} - {}",
                        q.name,
                        q.key_size,
                        q.entry_count,
                        q.key_type
                    );
                }

                let mut query_value_sizes = queries.clone();
                query_value_sizes.sort_by_key(|q| q.value_size);
                println!("\nLarge query values:");
                for q in query_value_sizes.iter().rev()
                                          .filter(|q| q.value_size > 8) {
                    println!(
                        "   {} - {} x {} - {}",
                        q.name,
                        q.value_size,
                        q.entry_count,
                        q.value_type
                    );
                }

                if cfg!(debug_assertions) {
                    let mut query_cache_hits = queries.clone();
                    query_cache_hits.sort_by_key(|q| q.cache_hits);
                    println!("\nQuery cache hits:");
                    for q in query_cache_hits.iter().rev() {
                        println!(
                            "   {} - {} ({}%)",
                            q.name,
                            q.cache_hits,
                            q.cache_hits as f64 / (q.cache_hits + q.entry_count) as f64
                        );
                    }
                }

                let mut query_value_count = queries.clone();
                query_value_count.sort_by_key(|q| q.entry_count);
                println!("\nQuery value count:");
                for q in query_value_count.iter().rev() {
                    println!("   {} - {}", q.name, q.entry_count);
                }
            }
        }
