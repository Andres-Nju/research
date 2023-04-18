File_Code/servo/b9d91929aa/restyle_hints/restyle_hints_after.rs --- Rust
473             debug_assert!(state_changes.intersects(dep.sensitivities.states) ||                                                                          473             debug_assert!((!state_changes.is_empty() && !dep.sensitivities.states.is_empty()) ||
474                           attrs_changed && dep.sensitivities.attrs,                                                                                      474                           (attrs_changed && dep.sensitivities.attrs),
475                           "Testing a completely ineffective dependency?");                                                                               475                           "Testing a known ineffective dependency?");
476             if !hint.intersects(dep.hint) {                                                                                                              476             if (attrs_changed || state_changes.intersects(dep.sensitivities.states)) && !hint.intersects(dep.hint) {

