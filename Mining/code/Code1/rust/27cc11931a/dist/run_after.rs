    fn run(self, builder: &Builder) -> PathBuf {
        let build = builder.build;
        let compiler = self.compiler;
        let target = self.target;
        assert!(build.config.extended);
        println!("Dist analysis");
        let name = pkgname(build, "rust-analysis");

        if &compiler.host != build.build {
            println!("\tskipping, not a build host");
            return distdir(build).join(format!("{}-{}.tar.gz", name, target));
        }

        builder.ensure(Std { compiler, target });

        // Package save-analysis from stage1 if not doing a full bootstrap, as the
        // stage2 artifacts is simply copied from stage1 in that case.
        let compiler = if build.force_use_stage1(compiler, target) {
            builder.compiler(1, compiler.host)
        } else {
            compiler.clone()
        };

        let image = tmpdir(build).join(format!("{}-{}-image", name, target));

        let src = build.stage_out(compiler, Mode::Libstd)
            .join(target).join(build.cargo_dir()).join("deps");

        let image_src = src.join("save-analysis");
        let dst = image.join("lib/rustlib").join(target).join("analysis");
        t!(fs::create_dir_all(&dst));
        println!("image_src: {:?}, dst: {:?}", image_src, dst);
        cp_r(&image_src, &dst);

        let mut cmd = rust_installer(builder);
        cmd.arg("generate")
           .arg("--product-name=Rust")
           .arg("--rel-manifest-dir=rustlib")
           .arg("--success-message=save-analysis-saved.")
           .arg("--image-dir").arg(&image)
           .arg("--work-dir").arg(&tmpdir(build))
           .arg("--output-dir").arg(&distdir(build))
           .arg(format!("--package-name={}-{}", name, target))
           .arg(format!("--component-name=rust-analysis-{}", target))
           .arg("--legacy-manifest-dirs=rustlib,cargo");
        build.run(&mut cmd);
        t!(fs::remove_dir_all(&image));
        distdir(build).join(format!("{}-{}.tar.gz", name, target))
    }
