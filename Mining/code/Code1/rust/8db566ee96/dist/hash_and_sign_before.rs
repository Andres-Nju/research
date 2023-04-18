pub fn hash_and_sign(build: &Build) {
    let compiler = Compiler::new(0, &build.config.build);
    let mut cmd = build.tool_cmd(&compiler, "build-manifest");
    let sign = build.config.dist_sign_folder.as_ref().unwrap_or_else(|| {
        panic!("\n\nfailed to specify `dist.sign-folder` in `config.toml`\n\n")
    });
    let addr = build.config.dist_upload_addr.as_ref().unwrap_or_else(|| {
        panic!("\n\nfailed to specify `dist.upload-addr` in `config.toml`\n\n")
    });
    let file = build.config.dist_gpg_password_file.as_ref().unwrap_or_else(|| {
        panic!("\n\nfailed to specify `dist.gpg-password-file` in `config.toml`\n\n")
    });
    let mut pass = String::new();
    t!(t!(File::open(&file)).read_to_string(&mut pass));

    let today = output(Command::new("date").arg("+%Y-%m-%d"));

    cmd.arg(sign);
    cmd.arg(distdir(build));
    cmd.arg(today.trim());
    cmd.arg(build.rust_package_vers());
    cmd.arg(build.cargo_info.version(build, &build.cargo_release_num()));
    cmd.arg(addr);

    t!(fs::create_dir_all(distdir(build)));

    let mut child = t!(cmd.stdin(Stdio::piped()).spawn());
    t!(child.stdin.take().unwrap().write_all(pass.as_bytes()));
    let status = t!(child.wait());
    assert!(status.success());
}
