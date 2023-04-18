    fn run(self, builder: &Builder) {
        let build = builder.build;

        let target = self.target;
        let name = self.name;
        let src = build.src.join("src/tools/cargo/src/doc/book");

        let out = build.doc_out(target);
        t!(fs::create_dir_all(&out));

        let out = out.join(name);

        println!("Cargo Book ({}) - {}", target, name);

        let _ = fs::remove_dir_all(&out);

        build.run(builder.tool_cmd(Tool::Rustbook)
                       .arg("build")
                       .arg(&src)
                       .arg("-d")
                       .arg(out));
    }
