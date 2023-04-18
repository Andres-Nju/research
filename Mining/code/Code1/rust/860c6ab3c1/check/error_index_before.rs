pub fn error_index(build: &Build, compiler: &Compiler) {
    println!("Testing error-index stage{}", compiler.stage);

    let output = testdir(build, compiler.host).join("error-index.md");
    build.run(build.tool_cmd(compiler, "error_index_generator")
                   .arg("markdown")
                   .arg(&output)
                   .env("CFG_BUILD", &build.config.build));

    markdown_test(build, compiler, &output);
}
