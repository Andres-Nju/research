File_Code/deno/6bbb4c3af6/path/path_after.rs --- 1/2 --- Rust
6 /// Extenion to path_clean::PathClean                                                                                                                      6 /// Extension to path_clean::PathClean

File_Code/deno/6bbb4c3af6/path/path_after.rs --- 2/2 --- Rust
25             let poped_component = components.pop();                                                                                                       25             let maybe_last_component = components.pop();
26             if !matches!(poped_component, Some(Component::Normal(_))) {                                                                                   26             if !matches!(maybe_last_component, Some(Component::Normal(_))) {

