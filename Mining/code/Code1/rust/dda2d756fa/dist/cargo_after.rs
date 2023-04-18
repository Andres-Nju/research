pub fn cargo(build: &Build, stage: u32, target: &str) {
    println!("Dist cargo stage{} ({})", stage, target);
    let compiler = Compiler::new(stage, &build.config.build);

    let src = build.src.join("src/tools/cargo");
    let etc = src.join("src/etc");
    let release_num = build.release_num("cargo");
    let name = pkgname(build, "cargo");
    let version = build.cargo_info.version(build, &release_num);

    let tmp = tmpdir(build);
    let image = tmp.join("cargo-image");
    drop(fs::remove_dir_all(&image));
    t!(fs::create_dir_all(&image));

    // Prepare the image directory
    t!(fs::create_dir_all(image.join("share/zsh/site-functions")));
    t!(fs::create_dir_all(image.join("etc/bash_completion.d")));
    let cargo = build.cargo_out(&compiler, Mode::Tool, target)
                     .join(exe("cargo", target));
    install(&cargo, &image.join("bin"), 0o755);
    for man in t!(etc.join("man").read_dir()) {
        let man = t!(man);
        install(&man.path(), &image.join("share/man/man1"), 0o644);
    }
    install(&etc.join("_cargo"), &image.join("share/zsh/site-functions"), 0o644);
    copy(&etc.join("cargo.bashcomp.sh"),
         &image.join("etc/bash_completion.d/cargo"));
    let doc = image.join("share/doc/cargo");
    install(&src.join("README.md"), &doc, 0o644);
    install(&src.join("LICENSE-MIT"), &doc, 0o644);
    install(&src.join("LICENSE-APACHE"), &doc, 0o644);
    install(&src.join("LICENSE-THIRD-PARTY"), &doc, 0o644);

    // Prepare the overlay
    let overlay = tmp.join("cargo-overlay");
    drop(fs::remove_dir_all(&overlay));
    t!(fs::create_dir_all(&overlay));
    install(&src.join("README.md"), &overlay, 0o644);
    install(&src.join("LICENSE-MIT"), &overlay, 0o644);
    install(&src.join("LICENSE-APACHE"), &overlay, 0o644);
    install(&src.join("LICENSE-THIRD-PARTY"), &overlay, 0o644);
    t!(t!(File::create(overlay.join("version"))).write_all(version.as_bytes()));

    // Generate the installer tarball
    let mut cmd = rust_installer(build);
    cmd.arg("generate")
       .arg("--product-name=Rust")
       .arg("--rel-manifest-dir=rustlib")
       .arg("--success-message=Rust-is-ready-to-roll.")
       .arg("--image-dir").arg(&image)
       .arg("--work-dir").arg(&tmpdir(build))
       .arg("--output-dir").arg(&distdir(build))
       .arg("--non-installed-overlay").arg(&overlay)
       .arg(format!("--package-name={}-{}", name, target))
       .arg("--component-name=cargo")
       .arg("--legacy-manifest-dirs=rustlib,cargo");
    build.run(&mut cmd);
}
