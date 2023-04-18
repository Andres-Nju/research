    fn substs_from_path(
        db: &impl HirDatabase,
        resolver: &Resolver,
        path: &Path,
        resolved: TypableDef,
    ) -> Substs {
        let mut substs = Vec::new();
        let last = path.segments.last().expect("path should have at least one segment");
        let (def_generics, segment) = match resolved {
            TypableDef::Function(func) => (func.generic_params(db), last),
            TypableDef::Struct(s) => (s.generic_params(db), last),
            TypableDef::Enum(e) => (e.generic_params(db), last),
            TypableDef::EnumVariant(var) => {
                // the generic args for an enum variant may be either specified
                // on the segment referring to the enum, or on the segment
                // referring to the variant. So `Option::<T>::None` and
                // `Option::None::<T>` are both allowed (though the former is
                // preferred). See also `def_ids_for_path_segments` in rustc.
                let len = path.segments.len();
                let segment = if len >= 2 && path.segments[len - 2].args_and_bindings.is_some() {
                    // Option::<T>::None
                    &path.segments[len - 2]
                } else {
                    // Option::None::<T>
                    last
                };
                (var.parent_enum(db).generic_params(db), segment)
            }
        };
        let parent_param_count = def_generics.count_parent_params();
        substs.extend((0..parent_param_count).map(|_| Ty::Unknown));
        if let Some(generic_args) = &segment.args_and_bindings {
            // if args are provided, it should be all of them, but we can't rely on that
            let param_count = def_generics.params.len();
            for arg in generic_args.args.iter().take(param_count) {
                match arg {
                    GenericArg::Type(type_ref) => {
                        let ty = Ty::from_hir(db, resolver, type_ref);
                        substs.push(ty);
                    }
                }
            }
        }
        // add placeholders for args that were not provided
        // TODO: handle defaults
        let supplied_params = substs.len();
        for _ in supplied_params..def_generics.count_params_including_parent() {
            substs.push(Ty::Unknown);
        }
        assert_eq!(substs.len(), def_generics.params.len());
        Substs(substs.into())
    }
