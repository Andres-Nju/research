fn maybe_redirect(source: &str) -> Option<String> {
    const REDIRECT: &'static str = "<p>Redirecting to <a href=";

    let mut lines = source.lines();
    let redirect_line = lines.nth(6)?;

    redirect_line.find(REDIRECT).map(|i| {
        let rest = &redirect_line[(i + REDIRECT.len() + 1)..];
        let pos_quote = rest.find('"').unwrap();
        rest[..pos_quote].to_owned()
    })
}

fn with_attrs_in_source<F: FnMut(&str, usize, &str)>(contents: &str, attr: &str, mut f: F) {
    let mut base = "";
    for (i, mut line) in contents.lines().enumerate() {
        while let Some(j) = line.find(attr) {
            let rest = &line[j + attr.len()..];
            // The base tag should always be the first link in the document so
            // we can get away with using one pass.
            let is_base = line[..j].ends_with("<base");
            line = rest;
            let pos_equals = match rest.find("=") {
                Some(i) => i,
                None => continue,
            };
            if rest[..pos_equals].trim_start_matches(" ") != "" {
                continue;
            }

            let rest = &rest[pos_equals + 1..];

            let pos_quote = match rest.find(&['"', '\''][..]) {
                Some(i) => i,
                None => continue,
            };
            let quote_delim = rest.as_bytes()[pos_quote] as char;

            if rest[..pos_quote].trim_start_matches(" ") != "" {
                continue;
            }
            let rest = &rest[pos_quote + 1..];
            let url = match rest.find(quote_delim) {
                Some(i) => &rest[..i],
                None => continue,
            };
            if is_base {
                base = url;
                continue;
            }
            f(url, i, base)
        }
