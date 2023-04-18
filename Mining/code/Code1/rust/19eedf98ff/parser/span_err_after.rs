    pub fn span_err<S: Into<MultiSpan>>(self,
                                        sp: S,
                                        handler: &errors::Handler) -> DiagnosticBuilder {
        match self {
            Error::FileNotFoundForModule { ref mod_name,
                                           ref default_path,
                                           ref secondary_path,
                                           ref dir_path } => {
                let mut err = struct_span_err!(handler, sp, E0583,
                                               "file not found for module `{}`", mod_name);
                err.help(&format!("name the file either {} or {} inside the directory \"{}\"",
                                  default_path,
                                  secondary_path,
                                  dir_path));
                err
            }
            Error::DuplicatePaths { ref mod_name, ref default_path, ref secondary_path } => {
                let mut err = struct_span_err!(handler, sp, E0584,
                                               "file for module `{}` found at both {} and {}",
                                               mod_name,
                                               default_path,
                                               secondary_path);
                err.help("delete or rename one of them to remove the ambiguity");
                err
            }
            Error::UselessDocComment => {
                let mut err = struct_span_err!(handler, sp, E0585,
                                  "found a documentation comment that doesn't document anything");
                err.help("doc comments must come before what they document, maybe a comment was \
                          intended with `//`?");
                err
            }
            Error::InclusiveRangeWithNoEnd => {
                let mut err = struct_span_err!(handler, sp, E0586,
                                               "inclusive range with no end");
                err.help("inclusive ranges must be bounded at the end (`..=b` or `a..=b`)");
                err
            }
        }
    }
