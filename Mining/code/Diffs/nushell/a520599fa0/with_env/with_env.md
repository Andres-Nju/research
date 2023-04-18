File_Code/nushell/a520599fa0/with_env/with_env_after.rs --- Rust
66                 example: r#"echo '{"X":"Y","W":"Z"}'|from json|with-env $it { echo $env.X $env.W }"#,                                                     66                 example: r#"echo '{"X":"Y","W":"Z"}'|from json|with-env $in { echo $env.X $env.W }"#,

