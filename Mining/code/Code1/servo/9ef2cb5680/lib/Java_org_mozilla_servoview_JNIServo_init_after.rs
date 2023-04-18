pub fn Java_org_mozilla_servoview_JNIServo_init(
    env: JNIEnv,
    _: JClass,
    activity: JObject,
    opts: JObject,
    callbacks_obj: JObject,
) {
    let (opts, log, log_str) = match get_options(&env, opts) {
        Ok((opts, log, log_str)) => (opts, log, log_str),
        Err(err) => {
            throw(&env, &err);
            return;
        },
    };

    if log {
        // Note: Android debug logs are stripped from a release build.
        // debug!() will only show in a debug build. Use info!() if logs
        // should show up in adb logcat with a release build.
        let filters = [
            "servo",
            "simpleservo",
            "simpleservo::jniapi",
            "simpleservo::gl_glue::egl",
            // Show JS errors by default.
            "script::dom::bindings::error",
            // Show GL errors by default.
            "canvas::webgl_thread",
            "compositing::compositor",
            "constellation::constellation",
        ];
        let mut filter = Filter::default().with_min_level(Level::Debug);
        for &module in &filters {
            filter = filter.with_allowed_module_path(module);
        }
        if let Some(log_str) = log_str {
            for module in log_str.split(',') {
                filter = filter.with_allowed_module_path(module);
            }
        }
        android_logger::init_once(filter, Some("simpleservo"));
    }

    info!("init");

    initialize_android_glue(&env, activity);
    redirect_stdout_to_logcat();

    let callbacks_ref = match env.new_global_ref(callbacks_obj) {
        Ok(r) => r,
        Err(_) => {
            throw(&env, "Failed to get global reference of callback argument");
            return;
        },
    };

    let wakeup = Box::new(WakeupCallback::new(callbacks_ref.clone(), &env));
    let callbacks = Box::new(HostCallbacks::new(callbacks_ref, &env));

    if let Err(err) =
        gl_glue::egl::init().and_then(|gl| simpleservo::init(opts, gl, wakeup, callbacks))
    {
        throw(&env, err)
    };
}
