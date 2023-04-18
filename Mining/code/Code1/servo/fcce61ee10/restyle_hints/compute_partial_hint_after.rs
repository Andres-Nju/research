    fn compute_partial_hint<E>(deps: &[Dependency],
                               element: &E,
                               snapshot: &ElementWrapper<E>,
                               state_changes: &ElementState,
                               attrs_changed: bool,
                               hint: &mut RestyleHint)
        where E: TElement,
    {
        if hint.is_all() {
            return;
        }
        for dep in deps {
            debug_assert!((!state_changes.is_empty() && !dep.sensitivities.states.is_empty()) ||
                          (attrs_changed && dep.sensitivities.attrs),
                          "Testing a known ineffective dependency?");
            if (attrs_changed || state_changes.intersects(dep.sensitivities.states)) && !hint.contains(dep.hint) {
                // We can ignore the selector flags, since they would have already been set during
                // original matching for any element that might change its matching behavior here.
                let matched_then =
                    matches_complex_selector(&dep.selector, snapshot, None,
                                             &mut StyleRelations::empty(),
                                             &mut |_, _| {});
                let matches_now =
                    matches_complex_selector(&dep.selector, element, None,
                                             &mut StyleRelations::empty(),
                                             &mut |_, _| {});
                if matched_then != matches_now {
                    hint.insert(dep.hint);
                }
                if hint.is_all() {
                    break;
                }
            }
        }
    }
