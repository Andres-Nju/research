pub fn opts() -> TargetOptions {
    TargetOptions {
        linker: "cc".to_string(),
        dynamic_linking: true,
        executables: true,
        has_rpath: false,
        target_family: Some("unix".to_string()),
        relro_level: RelroLevel::Full,
        linker_is_gnu: true,
        no_integrated_as: true,
        .. Default::default()
    }
}
