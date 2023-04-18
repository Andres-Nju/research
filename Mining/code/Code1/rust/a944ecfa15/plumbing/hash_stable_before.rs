            fn hash_stable<W: StableHasherResult>(&self,
                                                hcx: &mut StableHashingContext<'a>,
                                                hasher: &mut StableHasher<W>) {
                mem::discriminant(self).hash_stable(hcx, hasher);
                match *self {
                    $(Query::$name(key) => key.hash_stable(hcx, hasher),)*
                }
            }
        }

        pub mod queries {
            use std::marker::PhantomData;

            $(#[allow(nonstandard_style)]
            pub struct $name<$tcx> {
                data: PhantomData<&$tcx ()>
            })*
        }

        // This module and the functions in it exist only to provide a
        // predictable symbol name prefix for query providers. This is helpful
        // for analyzing queries in profilers.
        pub(super) mod __query_compute {
            $(#[inline(never)]
            pub fn $name<F: FnOnce() -> R, R>(f: F) -> R {
                f()
            })*
        }

        $(impl<$tcx> QueryConfig<$tcx> for queries::$name<$tcx> {
            type Key = $K;
            type Value = $V;

            const NAME: &'static str = stringify!($name);
            const CATEGORY: ProfileCategory = $category;
        }

        impl<$tcx> QueryAccessors<$tcx> for queries::$name<$tcx> {
            #[inline(always)]
            fn query(key: Self::Key) -> Query<'tcx> {
                Query::$name(key)
            }

            #[inline(always)]
            fn query_cache<'a>(tcx: TyCtxt<'a, $tcx, '_>) -> &'a Lock<QueryCache<$tcx, Self>> {
                &tcx.queries.$name
            }

            #[allow(unused)]
            #[inline(always)]
            fn to_dep_node(tcx: TyCtxt<'_, $tcx, '_>, key: &Self::Key) -> DepNode {
                use dep_graph::DepConstructor::*;

                DepNode::new(tcx, $node(*key))
            }

            #[inline]
            fn compute(tcx: TyCtxt<'_, 'tcx, '_>, key: Self::Key) -> Self::Value {
                __query_compute::$name(move || {
                    let provider = tcx.queries.providers.get(key.query_crate())
                        // HACK(eddyb) it's possible crates may be loaded after
                        // the query engine is created, and because crate loading
                        // is not yet integrated with the query engine, such crates
                        // would be be missing appropriate entries in `providers`.
                        .unwrap_or(&tcx.queries.fallback_extern_providers)
                        .$name;
                    provider(tcx.global_tcx(), key)
                })
            }
