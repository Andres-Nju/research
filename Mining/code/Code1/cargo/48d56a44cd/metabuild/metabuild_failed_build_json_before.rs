fn metabuild_failed_build_json() {
    let p = basic_project();
    // Modify the metabuild dep so that it fails to compile.
    p.change_file("mb/src/lib.rs", "");
    p.cargo("build --message-format=json")
        .masquerade_as_nightly_cargo()
        .with_status(101)
        .with_json_contains_unordered(
            r#"
{
  "message": {
    "children": "{...}",
    "code": "{...}",
    "level": "error",
    "message": "cannot find function `metabuild` in module `mb`",
    "rendered": "[..]",
    "spans": "{...}"
  },
  "package_id": "foo [..]",
  "reason": "compiler-message",
  "target": {
    "crate_types": [
      "bin"
    ],
    "edition": "2015",
    "kind": [
      "custom-build"
    ],
    "name": "metabuild-foo",
    "src_path": null
  }
}
"#,
        )
        .run();
}
