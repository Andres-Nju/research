    fn push_type_live_constraint<T>(&mut self, value: T, location: Location, cause: Cause)
    where
        T: TypeFoldable<'tcx>,
    {
        debug!(
            "push_type_live_constraint(live_ty={:?}, location={:?})",
            value,
            location
        );

        self.tcx.for_each_free_region(&value, |live_region| {
            self.cx
                .constraints
                .liveness_set
                .push((live_region, location, cause.clone()));
        });
    }

    /// Some variable with type `live_ty` is "drop live" at `location`
    /// -- i.e., it may be dropped later. This means that *some* of
    /// the regions in its type must be live at `location`. The
    /// precise set will depend on the dropck constraints, and in
    /// particular this takes `#[may_dangle]` into account.
    fn add_drop_live_constraint(
        &mut self,
        dropped_local: Local,
        dropped_ty: Ty<'tcx>,
        location: Location,
    ) {
        debug!(
            "add_drop_live_constraint(dropped_local={:?}, dropped_ty={:?}, location={:?})",
            dropped_local,
            dropped_ty,
            location
        );

        // If we end visiting the same type twice (usually due to a cycle involving
        // associated types), we need to ensure that its region types match up with the type
        // we added to the 'known' map the first time around. For this reason, we need
        // our infcx to hold onto its calculated region constraints after each call
        // to dtorck_constraint_for_ty. Otherwise, normalizing the corresponding associated
        // type will end up instantiating the type with a new set of inference variables
        // Since this new type will never be in 'known', we end up looping forever.
        //
        // For this reason, we avoid calling TypeChecker.normalize, instead doing all normalization
        // ourselves in one large 'fully_perform_op' callback.
        let (type_constraints, kind_constraints) = self.cx.fully_perform_op(location.at_self(),
            |cx| {

            let tcx = cx.infcx.tcx;
            let mut selcx = traits::SelectionContext::new(cx.infcx);
            let cause = cx.misc(cx.last_span);

            let mut types = vec![(dropped_ty, 0)];
            let mut final_obligations = Vec::new();
            let mut type_constraints = Vec::new();
            let mut kind_constraints = Vec::new();

            let mut known = FxHashSet();

            while let Some((ty, depth)) = types.pop() {
                let span = DUMMY_SP; // FIXME
                let result = match tcx.dtorck_constraint_for_ty(span, dropped_ty, depth, ty) {
                    Ok(result) => result,
                    Err(ErrorReported) => {
                        continue;
                    }
                };

                let ty::DtorckConstraint {
                    outlives,
                    dtorck_types,
                } = result;

                // All things in the `outlives` array may be touched by
                // the destructor and must be live at this point.
                for outlive in outlives {
                    let cause = Cause::DropVar(dropped_local, location);
                    kind_constraints.push((outlive, location, cause));
                }

                // However, there may also be some types that
                // `dtorck_constraint_for_ty` could not resolve (e.g.,
                // associated types and parameters). We need to normalize
                // associated types here and possibly recursively process.
                for ty in dtorck_types {
                    let traits::Normalized { value: ty, obligations } =
                        traits::normalize(&mut selcx, cx.param_env, cause.clone(), &ty);

                    final_obligations.extend(obligations);

                    //let ty = self.cx.normalize(&ty, location);
                    let ty = cx.infcx.resolve_type_and_region_vars_if_possible(&ty);
                    match ty.sty {
                        ty::TyParam(..) | ty::TyProjection(..) | ty::TyAnon(..) => {
                            let cause = Cause::DropVar(dropped_local, location);
                            type_constraints.push((ty, location, cause));
                        }

                        _ => if known.insert(ty) {
                            types.push((ty, depth + 1));
                        },
                    }
                }
            }

            Ok(InferOk {
                value: (type_constraints, kind_constraints), obligations: final_obligations
            })
        }).unwrap();

        for (ty, location, cause) in type_constraints {
            self.push_type_live_constraint(ty, location, cause);
        }

        for (kind, location, cause) in kind_constraints {
            self.push_type_live_constraint(kind, location, cause);
        }
    }
