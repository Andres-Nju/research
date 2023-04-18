    pub fn uninhabited_from(
                &self,
                visited: &mut FxHashMap<DefId, FxHashSet<&'tcx Substs<'tcx>>>,
                tcx: TyCtxt<'a, 'gcx, 'tcx>) -> DefIdForest
    {
        match tcx.lift_to_global(&self) {
            Some(global_ty) => {
                {
                    let cache = tcx.inhabitedness_cache.borrow();
                    if let Some(forest) = cache.get(&global_ty) {
                        return forest.clone();
                    }
                }
                let forest = global_ty.uninhabited_from_inner(visited, tcx);
                let mut cache = tcx.inhabitedness_cache.borrow_mut();
                cache.insert(global_ty, forest.clone());
                forest
            },
            None => {
                let forest = self.uninhabited_from_inner(visited, tcx);
                forest
            },
        }
    }

    fn uninhabited_from_inner(
                &self,
                visited: &mut FxHashMap<DefId, FxHashSet<&'tcx Substs<'tcx>>>,
                tcx: TyCtxt<'a, 'gcx, 'tcx>) -> DefIdForest
    {
        match self.sty {
            TyAdt(def, substs) => {
                {
                    let mut substs_set = visited.entry(def.did).or_insert(FxHashSet::default());
                    if !substs_set.insert(substs) {
                        // We are already calculating the inhabitedness of this type.
                        // The type must contain a reference to itself. Break the
                        // infinite loop.
                        return DefIdForest::empty();
                    }
                    if substs_set.len() >= tcx.sess.recursion_limit.get() / 4 {
                        // We have gone very deep, reinstantiating this ADT inside
                        // itself with different type arguments. We are probably
                        // hitting an infinite loop. For example, it's possible to write:
                        //                a type Foo<T>
                        //      which contains a Foo<(T, T)>
                        //      which contains a Foo<((T, T), (T, T))>
                        //      which contains a Foo<(((T, T), (T, T)), ((T, T), (T, T)))>
                        //      etc.
                        let error = format!("reached recursion limit while checking \
                                             inhabitedness of `{}`", self);
                        tcx.sess.fatal(&error);
                    }
                }
                let ret = def.uninhabited_from(visited, tcx, substs);
                let mut substs_set = visited.get_mut(&def.did).unwrap();
                substs_set.remove(substs);
                ret
            },

            TyNever => DefIdForest::full(tcx),
            TyTuple(ref tys, _) => {
                DefIdForest::union(tcx, tys.iter().map(|ty| {
                    ty.uninhabited_from(visited, tcx)
                }))
            },
            TyArray(ty, len) => {
                if len == 0 {
                    DefIdForest::empty()
                } else {
                    ty.uninhabited_from(visited, tcx)
                }
            }
            TyRef(_, ref tm) => {
                tm.ty.uninhabited_from(visited, tcx)
            }

            _ => DefIdForest::empty(),
        }
    }
