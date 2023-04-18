File_Code/nushell/df07be6a42/to_md/to_md_after.rs --- Rust
29         to_html(args, registry).await                                                                                                                     29         to_md(args, registry).await
30     }                                                                                                                                                     30     }
31 }                                                                                                                                                         31 }
32                                                                                                                                                           32 
33 async fn to_html(                                                                                                                                         33 async fn to_md(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
34     args: CommandArgs,                                                                                                                                       
35     registry: &CommandRegistry,                                                                                                                              

