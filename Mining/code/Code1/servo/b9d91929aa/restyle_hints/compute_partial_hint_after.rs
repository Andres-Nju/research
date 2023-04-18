    fn compute_partial_hint<E>(deps: &[Dependency],
                               element: &E,
                               snapshot: &ElementWrapper<E>,
                               state_changes: &ElementState,
                               attrs_changed: bool,
                               hint: &mut RestyleHint)
        where E: ElementExt
    {
        if hint.is_all() {
            return;
        }
        for dep in deps {
            debug_assert!((!state_changes.is_empty() && !dep.sensitivities.states.is_empty()) ||
                          (attrs_changed && dep.sensitivities.attrs),
                          "Testing a known ineffective dependency?");
            if (attrs_changed || state_changes.intersects(dep.sensitivities.states)) && !hint.intersects(dep.hint) {
                let matched_then =
                    matches_complex_selector(&dep.selector, snapshot, None,
                                             &mut StyleRelations::empty());
                let matches_now =
                    matches_complex_selector(&dep.selector, element, None,
                                             &mut StyleRelations::empty());
                if matched_then != matches_now {
                    hint.insert(dep.hint);
                }
                if hint.is_all() {
                    break;
                }
            }
        }
    }
