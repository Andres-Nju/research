fn new_store() -> Store {
    // Use empty enumset to disable simd.
    let mut set = EnumSet::new();
    set.insert(CpuFeature::SSE2);
    let target = Target::new(Triple::host(), set);

    let config = wasmer_compiler_cranelift::Cranelift::default();
    let engine = wasmer_engine_universal::Universal::new(config)
        .target(target)
        .engine();
    let tunables = BaseTunables::for_target(engine.target());
    Store::new_with_tunables(&engine, tunables)
}
