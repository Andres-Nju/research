fn parser_benchmarks(c: &mut Criterion) {
    let mut engine_state = nu_command::create_default_context();
    // parsing config.nu breaks without PWD set
    engine_state.add_env_var(
        "PWD".into(),
        Value::string("/some/dir".to_string(), Span::test_data()),
    );

    let default_env = get_default_env().as_bytes();
    c.bench_function("parse_default_env_file", |b| {
        b.iter_batched(
            || nu_protocol::engine::StateWorkingSet::new(&engine_state),
            |mut working_set| parse(&mut working_set, None, default_env, false, &[]),
            BatchSize::SmallInput,
        )
    });

    let default_config = get_default_config().as_bytes();
    c.bench_function("parse_default_config_file", |b| {
        b.iter_batched(
            || nu_protocol::engine::StateWorkingSet::new(&engine_state),
            |mut working_set| parse(&mut working_set, None, default_config, false, &[]),
            BatchSize::SmallInput,
        )
    });

    c.bench_function("eval default_env.nu", |b| {
        b.iter(|| {
            let mut engine_state = nu_command::create_default_context();
            let mut stack = nu_protocol::engine::Stack::new();
            eval_source(
                &mut engine_state,
                &mut stack,
                get_default_env().as_bytes(),
                "default_env.nu",
                PipelineData::empty(),
            )
        })
    });

    c.bench_function("eval default_config.nu", |b| {
        b.iter(|| {
            let mut engine_state = nu_command::create_default_context();
            // parsing config.nu breaks without PWD set
            engine_state.add_env_var(
                "PWD".into(),
                Value::string("/some/dir".to_string(), Span::test_data()),
            );
            let mut stack = nu_protocol::engine::Stack::new();
            eval_source(
                &mut engine_state,
                &mut stack,
                get_default_config().as_bytes(),
                "default_config.nu",
                PipelineData::empty(),
            )
        })
    });
}
