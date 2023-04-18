File_Code/deno/7ad0d09228/editor/editor_after.rs --- 1/2 --- Rust
374       KeyEvent::from('\t'),                                                                                                                              374       KeyEvent(KeyCode::Tab, Modifiers::NONE),

File_Code/deno/7ad0d09228/editor/editor_after.rs --- 2/2 --- Rust
414     debug_assert_eq!(*evt, Event::from(KeyEvent::from('\t')));                                                                                           414     debug_assert_eq!(
                                                                                                                                                             415       *evt,
                                                                                                                                                             416       Event::from(KeyEvent(KeyCode::Tab, Modifiers::NONE))

