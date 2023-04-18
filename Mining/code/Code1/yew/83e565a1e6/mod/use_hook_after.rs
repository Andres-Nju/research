pub fn use_hook<InternalHook: 'static, Output, Tear: FnOnce(&mut InternalHook) + 'static>(
    initializer: impl FnOnce() -> InternalHook,
    runner: impl FnOnce(&mut InternalHook, HookUpdater) -> Output,
    destructor: Tear,
) -> Output {
    if !CURRENT_HOOK.is_set() {
        panic!("Hooks can only be used in the scope of a function component");
    }

    // Extract current hook
    let updater = CURRENT_HOOK.with(|hook_state| {
        // Determine which hook position we're at and increment for the next hook
        let hook_pos = hook_state.counter;
        hook_state.counter += 1;

        // Initialize hook if this is the first call
        if hook_pos >= hook_state.hooks.len() {
            let initial_state = Rc::new(RefCell::new(initializer()));
            hook_state.hooks.push(initial_state.clone());
            hook_state.destroy_listeners.push(Box::new(move || {
                destructor(initial_state.borrow_mut().deref_mut());
            }));
        }

        let hook = hook_state
            .hooks
            .get(hook_pos)
            .expect("Not the same number of hooks. Hooks must not be called conditionally")
            .clone();

        HookUpdater {
            hook,
            process_message: hook_state.process_message.clone(),
        }
    });

    // Execute the actual hook closure we were given. Let it mutate the hook state and let
    // it create a callback that takes the mutable hook state.
    let mut hook = updater.hook.borrow_mut();
    let hook: &mut InternalHook = hook
        .downcast_mut()
        .expect("Incompatible hook type. Hooks must always be called in the same order");

    runner(hook, updater.clone())
}
