  fn test_set_flags_11() {
    let flags =
      flags_from_vec(svec!["deno", "-c", "tsconfig.json", "script.ts"]);
    assert_eq!(
      flags,
      DenoFlags {
        config_path: Some("tsconfig.json".to_owned()),
        ..DenoFlags::default()
      }
    )
  }
