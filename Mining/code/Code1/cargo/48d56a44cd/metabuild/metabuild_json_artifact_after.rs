fn metabuild_json_artifact() {
    let p = basic_project();
    p.cargo("build --message-format=json")
        .masquerade_as_nightly_cargo()
        .with_json_contains_unordered(
            r#"
{
  "executable": null,
  "features": [],
  "filenames": [
    "[..]/foo/target/debug/build/foo-[..]/metabuild-foo[EXE]"
  ],
  "fresh": false,
  "package_id": "foo [..]",
  "profile": "{...}",
  "reason": "compiler-artifact",
  "target": {
    "crate_types": [
      "bin"
    ],
    "edition": "2018",
    "kind": [
      "custom-build"
    ],
    "name": "metabuild-foo",
    "src_path": "[..]/foo/target/.metabuild/metabuild-foo-[..].rs"
  }
}

{
  "cfgs": [],
  "env": [],
  "linked_libs": [],
  "linked_paths": [],
  "package_id": "foo [..]",
  "reason": "build-script-executed"
}
"#,
        )
        .run();
}
