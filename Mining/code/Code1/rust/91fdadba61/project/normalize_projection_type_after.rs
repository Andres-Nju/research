pub fn normalize_projection_type<'a, 'b, 'gcx, 'tcx>(
    selcx: &'a mut SelectionContext<'b, 'gcx, 'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    projection_ty: ty::ProjectionTy<'tcx>,
    cause: ObligationCause<'tcx>,
    depth: usize)
    -> NormalizedTy<'tcx>
{
    opt_normalize_projection_type(selcx, param_env, projection_ty.clone(), cause.clone(), depth)
        .unwrap_or_else(move || {
            // if we bottom out in ambiguity, create a type variable
            // and a deferred predicate to resolve this when more type
            // information is available.

            let tcx = selcx.infcx().tcx;
            let def_id = projection_ty.item_def_id;
            let ty_var = selcx.infcx().next_ty_var(
                TypeVariableOrigin::NormalizeProjectionType(tcx.def_span(def_id)));
            let projection = ty::Binder(ty::ProjectionPredicate {
                projection_ty,
                ty: ty_var
            });
            let obligation = Obligation::with_depth(
                cause, depth + 1, param_env, projection.to_predicate());
            Normalized {
                value: ty_var,
                obligations: vec![obligation]
            }
        })
}

/// The guts of `normalize`: normalize a specific projection like `<T
/// as Trait>::Item`. The result is always a type (and possibly
/// additional obligations). Returns `None` in the case of ambiguity,
/// which indicates that there are unbound type variables.
fn opt_normalize_projection_type<'a, 'b, 'gcx, 'tcx>(
    selcx: &'a mut SelectionContext<'b, 'gcx, 'tcx>,
    param_env: ty::ParamEnv<'tcx>,
    projection_ty: ty::ProjectionTy<'tcx>,
    cause: ObligationCause<'tcx>,
    depth: usize)
    -> Option<NormalizedTy<'tcx>>
{
    let infcx = selcx.infcx();

    let projection_ty = infcx.resolve_type_vars_if_possible(&projection_ty);
    let cache_key = ProjectionCacheKey { ty: projection_ty };

    debug!("opt_normalize_projection_type(\
           projection_ty={:?}, \
           depth={})",
           projection_ty,
           depth);

    // FIXME(#20304) For now, I am caching here, which is good, but it
    // means we don't capture the type variables that are created in
    // the case of ambiguity. Which means we may create a large stream
    // of such variables. OTOH, if we move the caching up a level, we
    // would not benefit from caching when proving `T: Trait<U=Foo>`
    // bounds. It might be the case that we want two distinct caches,
    // or else another kind of cache entry.

    let cache_result = infcx.projection_cache.borrow_mut().try_start(cache_key);
    match cache_result {
        Ok(()) => { }
        Err(ProjectionCacheEntry::Ambiguous) => {
            // If we found ambiguity the last time, that generally
            // means we will continue to do so until some type in the
            // key changes (and we know it hasn't, because we just
            // fully resolved it). One exception though is closure
            // types, which can transition from having a fixed kind to
            // no kind with no visible change in the key.
            //
            // FIXME(#32286) refactor this so that closure type
            // changes
            debug!("opt_normalize_projection_type: \
                    found cache entry: ambiguous");
            if !projection_ty.has_closure_types() {
                return None;
            }
        }
        Err(ProjectionCacheEntry::InProgress) => {
            // If while normalized A::B, we are asked to normalize
            // A::B, just return A::B itself. This is a conservative
            // answer, in the sense that A::B *is* clearly equivalent
            // to A::B, though there may be a better value we can
            // find.

            // Under lazy normalization, this can arise when
            // bootstrapping.  That is, imagine an environment with a
            // where-clause like `A::B == u32`. Now, if we are asked
            // to normalize `A::B`, we will want to check the
            // where-clauses in scope. So we will try to unify `A::B`
            // with `A::B`, which can trigger a recursive
            // normalization. In that case, I think we will want this code:
            //
            // ```
            // let ty = selcx.tcx().mk_projection(projection_ty.item_def_id,
            //                                    projection_ty.substs;
            // return Some(NormalizedTy { value: v, obligations: vec![] });
            // ```

            debug!("opt_normalize_projection_type: \
                    found cache entry: in-progress");

            // But for now, let's classify this as an overflow:
            let recursion_limit = selcx.tcx().sess.recursion_limit.get();
            let obligation = Obligation::with_depth(cause.clone(),
                                                    recursion_limit,
                                                    param_env,
                                                    projection_ty);
            selcx.infcx().report_overflow_error(&obligation, false);
        }
        Err(ProjectionCacheEntry::NormalizedTy(mut ty)) => {
            // If we find the value in the cache, then return it along
            // with the obligations that went along with it. Note
            // that, when using a fulfillment context, these
            // obligations could in principle be ignored: they have
            // already been registered when the cache entry was
            // created (and hence the new ones will quickly be
            // discarded as duplicated). But when doing trait
            // evaluation this is not the case, and dropping the trait
            // evaluations can causes ICEs (e.g. #43132).
            debug!("opt_normalize_projection_type: \
                    found normalized ty `{:?}`",
                   ty);

            // Once we have inferred everything we need to know, we
            // can ignore the `obligations` from that point on.
            if !infcx.any_unresolved_type_vars(&ty.value) {
                infcx.projection_cache.borrow_mut().complete(cache_key);
                ty.obligations = vec![];
            }

            push_paranoid_cache_value_obligation(infcx,
                                                 param_env,
                                                 projection_ty,
                                                 cause,
                                                 depth,
                                                 &mut ty);

            return Some(ty);
        }
        Err(ProjectionCacheEntry::Error) => {
            debug!("opt_normalize_projection_type: \
                    found error");
            return Some(normalize_to_error(selcx, param_env, projection_ty, cause, depth));
        }
    }

    let obligation = Obligation::with_depth(cause.clone(), depth, param_env, projection_ty);
    match project_type(selcx, &obligation) {
        Ok(ProjectedTy::Progress(Progress { ty: projected_ty, mut obligations })) => {
            // if projection succeeded, then what we get out of this
            // is also non-normalized (consider: it was derived from
            // an impl, where-clause etc) and hence we must
            // re-normalize it

            debug!("opt_normalize_projection_type: \
                    projected_ty={:?} \
                    depth={} \
                    obligations={:?}",
                   projected_ty,
                   depth,
                   obligations);

            let result = if projected_ty.has_projections() {
                let mut normalizer = AssociatedTypeNormalizer::new(selcx,
                                                                   param_env,
                                                                   cause,
                                                                   depth+1);
                let normalized_ty = normalizer.fold(&projected_ty);

                debug!("opt_normalize_projection_type: \
                        normalized_ty={:?} depth={}",
                       normalized_ty,
                       depth);

                obligations.extend(normalizer.obligations);
                Normalized {
                    value: normalized_ty,
                    obligations,
                }
            } else {
                Normalized {
                    value: projected_ty,
                    obligations,
                }
            };

            let cache_value = prune_cache_value_obligations(infcx, &result);
            infcx.projection_cache.borrow_mut().insert_ty(cache_key, cache_value);

            Some(result)
        }
        Ok(ProjectedTy::NoProgress(projected_ty)) => {
            debug!("opt_normalize_projection_type: \
                    projected_ty={:?} no progress",
                   projected_ty);
            let result = Normalized {
                value: projected_ty,
                obligations: vec![]
            };
            infcx.projection_cache.borrow_mut().insert_ty(cache_key, result.clone());
            Some(result)
        }
        Err(ProjectionTyError::TooManyCandidates) => {
            debug!("opt_normalize_projection_type: \
                    too many candidates");
            infcx.projection_cache.borrow_mut()
                                  .ambiguous(cache_key);
            None
        }
        Err(ProjectionTyError::TraitSelectionError(_)) => {
            debug!("opt_normalize_projection_type: ERROR");
            // if we got an error processing the `T as Trait` part,
            // just return `ty::err` but add the obligation `T :
            // Trait`, which when processed will cause the error to be
            // reported later

            infcx.projection_cache.borrow_mut()
                                  .error(cache_key);
            Some(normalize_to_error(selcx, param_env, projection_ty, cause, depth))
        }
    }
}

/// If there are unresolved type variables, then we need to include
/// any subobligations that bind them, at least until those type
/// variables are fully resolved.
fn prune_cache_value_obligations<'a, 'gcx, 'tcx>(infcx: &'a InferCtxt<'a, 'gcx, 'tcx>,
                                                 result: &NormalizedTy<'tcx>)
                                                 -> NormalizedTy<'tcx> {
    if !infcx.any_unresolved_type_vars(&result.value) {
        return NormalizedTy { value: result.value, obligations: vec![] };
    }

    let mut obligations: Vec<_> =
        result.obligations
              .iter()
              .filter(|obligation| match obligation.predicate {
                  // We found a `T: Foo<X = U>` predicate, let's check
                  // if `U` references any unresolved type
                  // variables. In principle, we only care if this
                  // projection can help resolve any of the type
                  // variables found in `result.value` -- but we just
                  // check for any type variables here, for fear of
                  // indirect obligations (e.g., we project to `?0`,
                  // but we have `T: Foo<X = ?1>` and `?1: Bar<X =
                  // ?0>`).
                  ty::Predicate::Projection(ref data) =>
                      infcx.any_unresolved_type_vars(&data.ty()),

                  // We are only interested in `T: Foo<X = U>` predicates, whre
                  // `U` references one of `unresolved_type_vars`. =)
                  _ => false,
              })
              .cloned()
              .collect();

    obligations.shrink_to_fit();

    NormalizedTy { value: result.value, obligations }
