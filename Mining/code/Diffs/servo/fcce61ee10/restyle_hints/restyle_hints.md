File_Code/servo/fcce61ee10/restyle_hints/restyle_hints_after.rs --- Rust
687             if (attrs_changed || state_changes.intersects(dep.sensitivities.states)) && !hint.intersects(dep.hint) {                                     687             if (attrs_changed || state_changes.intersects(dep.sensitivities.states)) && !hint.contains(dep.hint) {

