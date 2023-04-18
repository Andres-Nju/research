fn write_minify_replacer<W: Write>(
    dst: &mut W,
    contents: &str,
    enable_minification: bool,
) -> io::Result<()> {
    use minifier::js::{simple_minify, Keyword, ReservedChar, Token, Tokens};

    if enable_minification {
        writeln!(dst, "{}",
                 {
                    let tokens: Tokens<'_> = simple_minify(contents)
                        .into_iter()
                        .filter(|f| {
                            // We keep backlines.
                            minifier::js::clean_token_except(f, &|c: &Token<'_>| {
                                c.get_char() != Some(ReservedChar::Backline)
                            })
                        })
                        .map(|f| {
                            minifier::js::replace_token_with(f, &|t: &Token<'_>| {
                                match *t {
                                    Token::Keyword(Keyword::Null) => Some(Token::Other("N")),
                                    Token::String(s) => {
                                        let s = &s[1..s.len() -1]; // The quotes are included
                                        if s.is_empty() {
                                            Some(Token::Other("E"))
                                        } else if s == "t" {
                                            Some(Token::Other("T"))
                                        } else if s == "u" {
                                            Some(Token::Other("U"))
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                }
                            })
                        })
                        .collect::<Vec<_>>()
                        .into();
                    tokens.apply(|f| {
                        // We add a backline after the newly created variables.
                        minifier::js::aggregate_strings_into_array_with_separation_filter(
                            f,
                            "R",
                            Token::Char(ReservedChar::Backline),
                            // This closure prevents crates' names from being aggregated.
                            //
                            // The point here is to check if the string is preceded by '[' and
                            // "searchIndex". If so, it means this is a crate name and that it
                            // shouldn't be aggregated.
                            |tokens, pos| {
                                pos < 2 ||
                                !tokens[pos - 1].is_char(ReservedChar::OpenBracket) ||
                                tokens[pos - 2].get_other() != Some("searchIndex")
                            }
                        )
                    })
                    .to_string()
                })
    } else {
