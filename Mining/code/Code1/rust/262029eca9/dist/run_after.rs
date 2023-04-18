    fn run(self, builder: &Builder) {
        let build = builder.build;
        let stage = self.stage;
        let target = self.target;

        println!("Dist extended stage{} ({})", stage, target);

        let rustc_installer = builder.ensure(Rustc {
            compiler: builder.compiler(stage, target),
        });
        let cargo_installer = builder.ensure(Cargo { stage, target });
        let rustfmt_installer = builder.ensure(Rustfmt { stage, target });
        let rls_installer = builder.ensure(Rls { stage, target });
        let mingw_installer = builder.ensure(Mingw { host: target });
        let analysis_installer = builder.ensure(Analysis {
            compiler: builder.compiler(stage, self.host),
            target
        });

        let docs_installer = builder.ensure(Docs { stage, host: target, });
        let std_installer = builder.ensure(Std {
            compiler: builder.compiler(stage, self.host),
            target,
        });

        let tmp = tmpdir(build);
        let overlay = tmp.join("extended-overlay");
        let etc = build.src.join("src/etc/installer");
        let work = tmp.join("work");

        let _ = fs::remove_dir_all(&overlay);
        install(&build.src.join("COPYRIGHT"), &overlay, 0o644);
        install(&build.src.join("LICENSE-APACHE"), &overlay, 0o644);
        install(&build.src.join("LICENSE-MIT"), &overlay, 0o644);
        let version = build.rust_version();
        t!(t!(File::create(overlay.join("version"))).write_all(version.as_bytes()));
        if let Some(sha) = build.rust_sha() {
            t!(t!(File::create(overlay.join("git-commit-hash"))).write_all(sha.as_bytes()));
        }
        install(&etc.join("README.md"), &overlay, 0o644);

        // When rust-std package split from rustc, we needed to ensure that during
        // upgrades rustc was upgraded before rust-std. To avoid rustc clobbering
        // the std files during uninstall. To do this ensure that rustc comes
        // before rust-std in the list below.
        let mut tarballs = Vec::new();
        tarballs.push(rustc_installer);
        tarballs.push(cargo_installer);
        tarballs.extend(rls_installer.clone());
        tarballs.extend(rustfmt_installer.clone());
        tarballs.push(analysis_installer);
        tarballs.push(std_installer);
        if build.config.docs {
            tarballs.push(docs_installer);
        }
        if target.contains("pc-windows-gnu") {
            tarballs.push(mingw_installer.unwrap());
        }
        let mut input_tarballs = tarballs[0].as_os_str().to_owned();
        for tarball in &tarballs[1..] {
            input_tarballs.push(",");
            input_tarballs.push(tarball);
        }

        let mut cmd = rust_installer(builder);
        cmd.arg("combine")
            .arg("--product-name=Rust")
            .arg("--rel-manifest-dir=rustlib")
            .arg("--success-message=Rust-is-ready-to-roll.")
            .arg("--work-dir").arg(&work)
            .arg("--output-dir").arg(&distdir(build))
            .arg(format!("--package-name={}-{}", pkgname(build, "rust"), target))
            .arg("--legacy-manifest-dirs=rustlib,cargo")
            .arg("--input-tarballs").arg(input_tarballs)
            .arg("--non-installed-overlay").arg(&overlay);
        build.run(&mut cmd);

        let mut license = String::new();
        t!(t!(File::open(build.src.join("COPYRIGHT"))).read_to_string(&mut license));
        license.push_str("\n");
        t!(t!(File::open(build.src.join("LICENSE-APACHE"))).read_to_string(&mut license));
        license.push_str("\n");
        t!(t!(File::open(build.src.join("LICENSE-MIT"))).read_to_string(&mut license));

        let rtf = r"{\rtf1\ansi\deff0{\fonttbl{\f0\fnil\fcharset0 Arial;}}\nowwrap\fs18";
        let mut rtf = rtf.to_string();
        rtf.push_str("\n");
        for line in license.lines() {
            rtf.push_str(line);
            rtf.push_str("\\line ");
        }
        rtf.push_str("}");

        fn filter(contents: &str, marker: &str) -> String {
            let start = format!("tool-{}-start", marker);
            let end = format!("tool-{}-end", marker);
            let mut lines = Vec::new();
            let mut omitted = false;
            for line in contents.lines() {
                if line.contains(&start) {
                    omitted = true;
                } else if line.contains(&end) {
                    omitted = false;
                } else if !omitted {
                    lines.push(line);
                }
            }

            lines.join("\n")
        }

        let xform = |p: &Path| {
            let mut contents = String::new();
            t!(t!(File::open(p)).read_to_string(&mut contents));
            if rls_installer.is_none() {
                contents = filter(&contents, "rls");
            }
            if rustfmt_installer.is_none() {
                contents = filter(&contents, "rustfmt");
            }
            let ret = tmp.join(p.file_name().unwrap());
            t!(t!(File::create(&ret)).write_all(contents.as_bytes()));
            return ret
        };

        if target.contains("apple-darwin") {
            let pkg = tmp.join("pkg");
            let _ = fs::remove_dir_all(&pkg);

            let pkgbuild = |component: &str| {
                let mut cmd = Command::new("pkgbuild");
                cmd.arg("--identifier").arg(format!("org.rust-lang.{}", component))
                    .arg("--scripts").arg(pkg.join(component))
                    .arg("--nopayload")
                    .arg(pkg.join(component).with_extension("pkg"));
                build.run(&mut cmd);
            };

            let prepare = |name: &str| {
                t!(fs::create_dir_all(pkg.join(name)));
                cp_r(&work.join(&format!("{}-{}", pkgname(build, name), target)),
                        &pkg.join(name));
                install(&etc.join("pkg/postinstall"), &pkg.join(name), 0o755);
                pkgbuild(name);
            };
            prepare("rustc");
            prepare("cargo");
            prepare("rust-docs");
            prepare("rust-std");
            prepare("rust-analysis");

            if rls_installer.is_some() {
                prepare("rls");
            }

            // create an 'uninstall' package
            install(&etc.join("pkg/postinstall"), &pkg.join("uninstall"), 0o755);
            pkgbuild("uninstall");

            t!(fs::create_dir_all(pkg.join("res")));
            t!(t!(File::create(pkg.join("res/LICENSE.txt"))).write_all(license.as_bytes()));
            install(&etc.join("gfx/rust-logo.png"), &pkg.join("res"), 0o644);
            let mut cmd = Command::new("productbuild");
            cmd.arg("--distribution").arg(xform(&etc.join("pkg/Distribution.xml")))
                .arg("--resources").arg(pkg.join("res"))
                .arg(distdir(build).join(format!("{}-{}.pkg",
                                                    pkgname(build, "rust"),
                                                    target)))
                .arg("--package-path").arg(&pkg);
            build.run(&mut cmd);
        }

        if target.contains("windows") {
            let exe = tmp.join("exe");
            let _ = fs::remove_dir_all(&exe);

            let prepare = |name: &str| {
                t!(fs::create_dir_all(exe.join(name)));
                let dir = if name == "rust-std" || name == "rust-analysis" {
                    format!("{}-{}", name, target)
                } else if name == "rls" {
                    "rls-preview".to_string()
                } else {
                    name.to_string()
                };
                cp_r(&work.join(&format!("{}-{}", pkgname(build, name), target))
                            .join(dir),
                        &exe.join(name));
                t!(fs::remove_file(exe.join(name).join("manifest.in")));
            };
            prepare("rustc");
            prepare("cargo");
            prepare("rust-analysis");
            prepare("rust-docs");
            prepare("rust-std");
            if rls_installer.is_some() {
                prepare("rls");
            }
            if target.contains("windows-gnu") {
                prepare("rust-mingw");
            }

            install(&xform(&etc.join("exe/rust.iss")), &exe, 0o644);
            install(&etc.join("exe/modpath.iss"), &exe, 0o644);
            install(&etc.join("exe/upgrade.iss"), &exe, 0o644);
            install(&etc.join("gfx/rust-logo.ico"), &exe, 0o644);
            t!(t!(File::create(exe.join("LICENSE.txt"))).write_all(license.as_bytes()));

            // Generate exe installer
            let mut cmd = Command::new("iscc");
            cmd.arg("rust.iss")
                .current_dir(&exe);
            if target.contains("windows-gnu") {
                cmd.arg("/dMINGW");
            }
            add_env(build, &mut cmd, target);
            build.run(&mut cmd);
            install(&exe.join(format!("{}-{}.exe", pkgname(build, "rust"), target)),
                    &distdir(build),
                    0o755);

            // Generate msi installer
            let wix = PathBuf::from(env::var_os("WIX").unwrap());
            let heat = wix.join("bin/heat.exe");
            let candle = wix.join("bin/candle.exe");
            let light = wix.join("bin/light.exe");

            let heat_flags = ["-nologo", "-gg", "-sfrag", "-srd", "-sreg"];
            build.run(Command::new(&heat)
                            .current_dir(&exe)
                            .arg("dir")
                            .arg("rustc")
                            .args(&heat_flags)
                            .arg("-cg").arg("RustcGroup")
                            .arg("-dr").arg("Rustc")
                            .arg("-var").arg("var.RustcDir")
                            .arg("-out").arg(exe.join("RustcGroup.wxs")));
            build.run(Command::new(&heat)
                            .current_dir(&exe)
                            .arg("dir")
                            .arg("rust-docs")
                            .args(&heat_flags)
                            .arg("-cg").arg("DocsGroup")
                            .arg("-dr").arg("Docs")
                            .arg("-var").arg("var.DocsDir")
                            .arg("-out").arg(exe.join("DocsGroup.wxs"))
                            .arg("-t").arg(etc.join("msi/squash-components.xsl")));
            build.run(Command::new(&heat)
                            .current_dir(&exe)
                            .arg("dir")
                            .arg("cargo")
                            .args(&heat_flags)
                            .arg("-cg").arg("CargoGroup")
                            .arg("-dr").arg("Cargo")
                            .arg("-var").arg("var.CargoDir")
                            .arg("-out").arg(exe.join("CargoGroup.wxs"))
                            .arg("-t").arg(etc.join("msi/remove-duplicates.xsl")));
            build.run(Command::new(&heat)
                            .current_dir(&exe)
                            .arg("dir")
                            .arg("rust-std")
                            .args(&heat_flags)
                            .arg("-cg").arg("StdGroup")
                            .arg("-dr").arg("Std")
                            .arg("-var").arg("var.StdDir")
                            .arg("-out").arg(exe.join("StdGroup.wxs")));
            if rls_installer.is_some() {
                build.run(Command::new(&heat)
                                .current_dir(&exe)
                                .arg("dir")
                                .arg("rls")
                                .args(&heat_flags)
                                .arg("-cg").arg("RlsGroup")
                                .arg("-dr").arg("Rls")
                                .arg("-var").arg("var.RlsDir")
                                .arg("-out").arg(exe.join("RlsGroup.wxs"))
                                .arg("-t").arg(etc.join("msi/remove-duplicates.xsl")));
            }
            build.run(Command::new(&heat)
                            .current_dir(&exe)
                            .arg("dir")
                            .arg("rust-analysis")
                            .args(&heat_flags)
                            .arg("-cg").arg("AnalysisGroup")
                            .arg("-dr").arg("Analysis")
                            .arg("-var").arg("var.AnalysisDir")
                            .arg("-out").arg(exe.join("AnalysisGroup.wxs"))
                            .arg("-t").arg(etc.join("msi/remove-duplicates.xsl")));
            if target.contains("windows-gnu") {
                build.run(Command::new(&heat)
                                .current_dir(&exe)
                                .arg("dir")
                                .arg("rust-mingw")
                                .args(&heat_flags)
                                .arg("-cg").arg("GccGroup")
                                .arg("-dr").arg("Gcc")
                                .arg("-var").arg("var.GccDir")
                                .arg("-out").arg(exe.join("GccGroup.wxs")));
            }

            let candle = |input: &Path| {
                let output = exe.join(input.file_stem().unwrap())
                                .with_extension("wixobj");
                let arch = if target.contains("x86_64") {"x64"} else {"x86"};
                let mut cmd = Command::new(&candle);
                cmd.current_dir(&exe)
                    .arg("-nologo")
                    .arg("-dRustcDir=rustc")
                    .arg("-dDocsDir=rust-docs")
                    .arg("-dCargoDir=cargo")
                    .arg("-dStdDir=rust-std")
                    .arg("-dAnalysisDir=rust-analysis")
                    .arg("-arch").arg(&arch)
                    .arg("-out").arg(&output)
                    .arg(&input);
                add_env(build, &mut cmd, target);

                if rls_installer.is_some() {
                    cmd.arg("-dRlsDir=rls");
                }
                if target.contains("windows-gnu") {
                    cmd.arg("-dGccDir=rust-mingw");
                }
                build.run(&mut cmd);
            };
            candle(&xform(&etc.join("msi/rust.wxs")));
            candle(&etc.join("msi/ui.wxs"));
            candle(&etc.join("msi/rustwelcomedlg.wxs"));
            candle("RustcGroup.wxs".as_ref());
            candle("DocsGroup.wxs".as_ref());
            candle("CargoGroup.wxs".as_ref());
            candle("StdGroup.wxs".as_ref());
            if rls_installer.is_some() {
                candle("RlsGroup.wxs".as_ref());
            }
            candle("AnalysisGroup.wxs".as_ref());

            if target.contains("windows-gnu") {
                candle("GccGroup.wxs".as_ref());
            }

            t!(t!(File::create(exe.join("LICENSE.rtf"))).write_all(rtf.as_bytes()));
            install(&etc.join("gfx/banner.bmp"), &exe, 0o644);
            install(&etc.join("gfx/dialogbg.bmp"), &exe, 0o644);

            let filename = format!("{}-{}.msi", pkgname(build, "rust"), target);
            let mut cmd = Command::new(&light);
            cmd.arg("-nologo")
                .arg("-ext").arg("WixUIExtension")
                .arg("-ext").arg("WixUtilExtension")
                .arg("-out").arg(exe.join(&filename))
                .arg("rust.wixobj")
                .arg("ui.wixobj")
                .arg("rustwelcomedlg.wixobj")
                .arg("RustcGroup.wixobj")
                .arg("DocsGroup.wixobj")
                .arg("CargoGroup.wixobj")
                .arg("StdGroup.wixobj")
                .arg("AnalysisGroup.wixobj")
                .current_dir(&exe);

            if rls_installer.is_some() {
                cmd.arg("RlsGroup.wixobj");
            }

            if target.contains("windows-gnu") {
                cmd.arg("GccGroup.wixobj");
            }
            // ICE57 wrongly complains about the shortcuts
            cmd.arg("-sice:ICE57");

            build.run(&mut cmd);

            t!(fs::rename(exe.join(&filename), distdir(build).join(&filename)));
        }
    }
