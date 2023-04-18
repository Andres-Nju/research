pub fn render_with_highlighting(src: &str, class: Option<&str>,
                                extension: Option<&str>,
                                tooltip: Option<(&str, &str)>) -> String {
    debug!("highlighting: ================\n{}\n==============", src);
    let sess = parse::ParseSess::new(FilePathMapping::empty());
    let fm = sess.source_map().new_source_file(FileName::Custom("stdin".to_string()),
                                               src.to_string());

    let mut out = Vec::new();
    if let Some((tooltip, class)) = tooltip {
        write!(out, "<div class='information'><div class='tooltip {}'>ⓘ<span \
                     class='tooltiptext'>{}</span></div></div>",
               class, tooltip).unwrap();
    }
    write_header(class, &mut out).unwrap();

    let lexer = match lexer::StringReader::new_without_err(&sess, fm, None, "Output from rustc:") {
        Ok(l) => l,
        Err(_) => {
            let first_line = src.lines().next().unwrap_or_else(|| "");
            let mut err = sess.span_diagnostic
                              .struct_warn(&format!("Invalid doc comment starting with: `{}`\n\
                                                     (Ignoring this codeblock)",
                                                    first_line));
            err.emit();
            return String::new();
        }
    };
    let mut classifier = Classifier::new(lexer, sess.source_map());
    if classifier.write_source(&mut out).is_err() {
        classifier.lexer.emit_fatal_errors();
        return format!("<pre>{}</pre>", src);
    }

    if let Some(extension) = extension {
        write!(out, "{}", extension).unwrap();
    }
    write_footer(&mut out).unwrap();
    String::from_utf8_lossy(&out[..]).into_owned()
}
