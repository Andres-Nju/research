pub fn cargo(build: &Build, stage: u32, target: &str) {
    println!("Dist cargo stage{} ({})", stage, target);

    let branch = match &build.config.channel[..] {
        "stable" |
        "beta" => format!("rust-{}", build.release_num),
        _ => "master".to_string(),
    };

    let dst = tmpdir(build).join("cargo");
    let _ = fs::remove_dir_all(&dst);
    build.run(Command::new("git")
                .arg("clone")
                .arg("--depth").arg("1")
                .arg("--branch").arg(&branch)
                .arg("https://github.com/rust-lang/cargo")
                .current_dir(dst.parent().unwrap()));
    let sha = output(Command::new("git")
                .arg("rev-parse")
                .arg("HEAD")
                .current_dir(&dst));
    let sha = sha.trim();
    println!("\tgot cargo sha: {}", sha);

    let input = format!("https://s3.amazonaws.com/rust-lang-ci/cargo-builds\
                         /{}/cargo-nightly-{}.tar.gz", sha, target);
    let output = distdir(build).join(format!("cargo-nightly-{}.tar.gz", target));
    println!("\tdownloading {}", input);
    let mut curl = Command::new("curl");
    curl.arg("-f")
        .arg("-o").arg(&output)
        .arg(&input)
        .arg("--retry").arg("3");
    build.run(&mut curl);
}
