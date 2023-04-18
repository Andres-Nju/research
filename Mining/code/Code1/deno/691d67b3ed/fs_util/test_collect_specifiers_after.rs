  fn test_collect_specifiers() {
    fn create_files(dir_path: &Path, files: &[&str]) {
      std::fs::create_dir(dir_path).expect("Failed to create directory");
      for f in files {
        let path = dir_path.join(f);
        std::fs::write(path, "").expect("Failed to create file");
      }
    }

    // dir.ts
    // ├── a.ts
    // ├── b.js
    // ├── child
    // │   ├── e.mjs
    // │   ├── f.mjsx
    // │   ├── .foo.TS
    // │   └── README.md
    // ├── c.tsx
    // ├── d.jsx
    // └── ignore
    //     ├── g.d.ts
    //     └── .gitignore

    let t = TempDir::new();

    let root_dir_path = t.path().join("dir.ts");
    let root_dir_files = ["a.ts", "b.js", "c.tsx", "d.jsx"];
    create_files(&root_dir_path, &root_dir_files);

    let child_dir_path = root_dir_path.join("child");
    let child_dir_files = ["e.mjs", "f.mjsx", ".foo.TS", "README.md"];
    create_files(&child_dir_path, &child_dir_files);

    let ignore_dir_path = root_dir_path.join("ignore");
    let ignore_dir_files = ["g.d.ts", ".gitignore"];
    create_files(&ignore_dir_path, &ignore_dir_files);

    let predicate = |path: &Path| {
      // exclude dotfiles
      path
        .file_name()
        .and_then(|f| f.to_str())
        .map_or(false, |f| !f.starts_with('.'))
    };

    let result = collect_specifiers(
      vec![
        "http://localhost:8080".to_string(),
        root_dir_path.to_str().unwrap().to_string(),
        "https://localhost:8080".to_string(),
      ],
      &[ignore_dir_path],
      predicate,
    )
    .unwrap();

    let root_dir_url = ModuleSpecifier::from_file_path(
      canonicalize_path(&root_dir_path).unwrap(),
    )
    .unwrap()
    .to_string();
    let expected: Vec<ModuleSpecifier> = [
      "http://localhost:8080",
      &format!("{}/a.ts", root_dir_url),
      &format!("{}/b.js", root_dir_url),
      &format!("{}/c.tsx", root_dir_url),
      &format!("{}/child/README.md", root_dir_url),
      &format!("{}/child/e.mjs", root_dir_url),
      &format!("{}/child/f.mjsx", root_dir_url),
      &format!("{}/d.jsx", root_dir_url),
      "https://localhost:8080",
    ]
    .iter()
    .map(|f| ModuleSpecifier::parse(f).unwrap())
    .collect::<Vec<_>>();

    assert_eq!(result, expected);

    let scheme = if cfg!(target_os = "windows") {
      "file:///"
    } else {
      "file://"
    };
    let result = collect_specifiers(
      vec![format!(
        "{}{}",
        scheme,
        root_dir_path
          .join("child")
          .to_str()
          .unwrap()
          .replace('\\', "/")
      )],
      &[],
      predicate,
    )
    .unwrap();

    let expected: Vec<ModuleSpecifier> = [
      &format!("{}/child/README.md", root_dir_url),
      &format!("{}/child/e.mjs", root_dir_url),
      &format!("{}/child/f.mjsx", root_dir_url),
    ]
    .iter()
    .map(|f| ModuleSpecifier::parse(f).unwrap())
    .collect::<Vec<_>>();

    assert_eq!(result, expected);
  }
