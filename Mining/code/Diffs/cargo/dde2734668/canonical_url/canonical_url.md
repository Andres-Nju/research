File_Code/cargo/dde2734668/canonical_url/canonical_url_after.rs --- Rust
42             url.set_scheme("https").unwrap();                                                                                                             42             url = format!("https{}", &url[url::Position::AfterScheme..])
                                                                                                                                                             43                 .parse()
                                                                                                                                                             44                 .unwrap();

