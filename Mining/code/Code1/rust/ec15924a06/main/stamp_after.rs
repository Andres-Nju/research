fn stamp(config: &Config, testpaths: &TestPaths) -> PathBuf {
    let stamp_name = format!("{}-{}.stamp",
                             testpaths.file.file_name().unwrap()
                                           .to_str().unwrap(),
                             config.stage_id);
    config.build_base.canonicalize()
          .unwrap_or_else(|_| config.build_base.clone())
          .join(&testpaths.relative_dir)
          .join(stamp_name)
}
