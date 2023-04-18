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

            pub fn record_computed_queries(&self, sess: &Session) {
                sess.profiler(|p| {
                    $(
                        p.record_computed_queries(
                            <queries::$name<'_> as QueryConfig<'_>>::CATEGORY,
                            self.$name.lock().results.len()
                        );
                    )*
                });
            }

            #[cfg(parallel_queries)]
            pub fn collect_active_jobs(&self) -> Vec<Lrc<QueryJob<$tcx>>> {
                let mut jobs = Vec::new();

                // We use try_lock here since we are only called from the
                // deadlock handler, and this shouldn't be locked
                $(
                    jobs.extend(
                        self.$name.try_lock().unwrap().active.values().filter_map(|v|
                            if let QueryResult::Started(ref job) = *v {
                                Some(job.clone())
                            } else {
                                None
                            }
                        )
                    );
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
                    map: &QueryCache<'tcx, Q>
                ) -> QueryStats {
                    QueryStats {
                        name,
                        #[cfg(debug_assertions)]
                        cache_hits: map.cache_hits,
                        #[cfg(not(debug_assertions))]
                        cache_hits: 0,
                        key_size: mem::size_of::<Q::Key>(),
                        key_type: unsafe { type_name::<Q::Key>() },
                        value_size: mem::size_of::<Q::Value>(),
                        value_type: unsafe { type_name::<Q::Value>() },
                        entry_count: map.results.len(),
                    }
                }

                $(
                    queries.push(stats::<queries::$name<'_>>(
                        stringify!($name),
                        &*self.$name.lock()
                    ));
                )*

                if cfg!(debug_assertions) {
                    let hits: usize = queries.iter().map(|s| s.cache_hits).sum();
                    let results: usize = queries.iter().map(|s| s.entry_count).sum();
                    println!("\nQuery cache hit rate: {}", hits as f64 / (hits + results) as f64);
                }
