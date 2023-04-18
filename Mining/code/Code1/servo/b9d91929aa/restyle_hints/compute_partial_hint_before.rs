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
            debug_assert!(state_changes.intersects(dep.sensitivities.states) ||
                          attrs_changed && dep.sensitivities.attrs,
                          "Testing a completely ineffective dependency?");
            if !hint.intersects(dep.hint) {
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
