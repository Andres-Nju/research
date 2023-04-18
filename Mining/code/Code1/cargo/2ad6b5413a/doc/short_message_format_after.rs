fn short_message_format() {
    if !is_nightly() {
        // This can be removed once intra_doc_link_resolution_failure fails on stable.
        return;
    }
    let p = project().file("src/lib.rs", BAD_INTRA_LINK_LIB).build();
    p.cargo("doc --message-format=short")
        .with_status(101)
        .with_stderr_contains("src/lib.rs:4:6: error: `[bad_link]` cannot be resolved[..]")
        .run();
}
