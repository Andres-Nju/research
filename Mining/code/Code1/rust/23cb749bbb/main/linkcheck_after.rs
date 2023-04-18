pub fn linkcheck(
    args: &ArgMatches<'_>,
) -> Result<(Vec<codespan_reporting::diagnostic::Diagnostic>, codespan::Files), failure::Error> {
    use mdbook_linkcheck::Reason;

    let book_dir = get_book_dir(args);
    let src_dir = book_dir.join("src");
    let book = MDBook::load(&book_dir).unwrap();
    let linkck_cfg = mdbook_linkcheck::get_config(&book.config)?;
    let mut files = codespan::Files::new();
    let target_files = mdbook_linkcheck::load_files_into_memory(&book.book, &mut files);
    let cache = mdbook_linkcheck::Cache::default();

    let (links, incomplete) = mdbook_linkcheck::extract_links(target_files, &files);

    let outcome =
        mdbook_linkcheck::validate(&links, &linkck_cfg, &src_dir, &cache, &files, incomplete)?;

    let mut is_real_error = false;

    for link in outcome.invalid_links.iter() {
        match &link.reason {
            Reason::FileNotFound | Reason::TraversesParentDirectories => {
                is_real_error = true;
            }
            Reason::UnsuccessfulServerResponse(status) => {
                if status.is_client_error() {
                    is_real_error = true;
                } else {
                    eprintln!("Unsuccessful server response for link `{}`", link.link.uri);
                }
            }
            Reason::Client(err) => {
                if err.is_timeout() {
                    eprintln!("Timeout for link `{}`", link.link.uri);
                } else if err.is_server_error() {
                    eprintln!("Server error for link `{}`", link.link.uri);
                } else if !err.is_http() {
                    eprintln!("Non-HTTP-related error for link: {} {}", link.link.uri, err);
                } else {
                    is_real_error = true;
                }
            }
        }
    }

    if is_real_error {
        Ok((outcome.generate_diagnostics(&files, linkck_cfg.warning_policy), files))
    } else {
        Ok((vec![], files))
    }
}

// Build command implementation
pub fn build(args: &ArgMatches<'_>) -> Result3<()> {
    let book_dir = get_book_dir(args);
    let mut book = MDBook::load(&book_dir)?;

    // Set this to allow us to catch bugs in advance.
    book.config.build.create_missing = false;

    if let Some(dest_dir) = args.value_of("dest-dir") {
        book.config.build.build_dir = PathBuf::from(dest_dir);
    }

    book.build()?;

    Ok(())
}
