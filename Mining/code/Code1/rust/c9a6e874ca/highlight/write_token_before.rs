    fn write_token<W: Writer>(&mut self,
                              out: &mut W,
                              tas: TokenAndSpan)
                              -> io::Result<()> {
        let klass = match tas.tok {
            token::Shebang(s) => {
                out.string(Escape(&s.as_str()), Class::None, Some(&tas))?;
                return Ok(());
            },

            token::Whitespace => Class::None,
            token::Comment => Class::Comment,
            token::DocComment(..) => Class::DocComment,

            // If this '&' token is directly adjacent to another token, assume
            // that it's the address-of operator instead of the and-operator.
            token::BinOp(token::And) if self.lexer.peek().sp.lo == tas.sp.hi => Class::RefKeyWord,

            // Consider this as part of a macro invocation if there was a
            // leading identifier.
            token::Not if self.in_macro => {
                self.in_macro = false;
                Class::Macro
            }

            // Operators.
            token::Eq | token::Lt | token::Le | token::EqEq | token::Ne | token::Ge | token::Gt |
                token::AndAnd | token::OrOr | token::Not | token::BinOp(..) | token::RArrow |
                token::BinOpEq(..) | token::FatArrow => Class::Op,

            // Miscellaneous, no highlighting.
            token::Dot | token::DotDot | token::DotDotDot | token::Comma | token::Semi |
                token::Colon | token::ModSep | token::LArrow | token::OpenDelim(_) |
                token::CloseDelim(token::Brace) | token::CloseDelim(token::Paren) |
                token::CloseDelim(token::NoDelim) => Class::None,

            token::Question => Class::QuestionMark,

            token::Dollar => {
                if self.lexer.peek().tok.is_ident() {
                    self.in_macro_nonterminal = true;
                    Class::MacroNonTerminal
                } else {
                    Class::None
                }
            }

            // This is the start of an attribute. We're going to want to
            // continue highlighting it as an attribute until the ending ']' is
            // seen, so skip out early. Down below we terminate the attribute
            // span when we see the ']'.
            token::Pound => {
                self.in_attribute = true;
                out.enter_span(Class::Attribute)?;
                out.string("#", Class::None, None)?;
                return Ok(());
            }
            token::CloseDelim(token::Bracket) => {
                if self.in_attribute {
                    self.in_attribute = false;
                    out.string("]", Class::None, None)?;
                    out.exit_span()?;
                    return Ok(());
                } else {
                    Class::None
                }
            }

            token::Literal(lit, _suf) => {
                match lit {
                    // Text literals.
                    token::Byte(..) | token::Char(..) |
                        token::ByteStr(..) | token::ByteStrRaw(..) |
                        token::Str_(..) | token::StrRaw(..) => Class::String,

                    // Number literals.
                    token::Integer(..) | token::Float(..) => Class::Number,
                }
            }

            // Keywords are also included in the identifier set.
            token::Ident(ident) => {
                match &*ident.name.as_str() {
                    "ref" | "mut" => Class::RefKeyWord,

                    "self" |"Self" => Class::Self_,
                    "false" | "true" => Class::Bool,

                    "Option" | "Result" => Class::PreludeTy,
                    "Some" | "None" | "Ok" | "Err" => Class::PreludeVal,

                    "$crate" => Class::KeyWord,
                    _ if tas.tok.is_any_keyword() => Class::KeyWord,

                    _ => {
                        if self.in_macro_nonterminal {
                            self.in_macro_nonterminal = false;
                            Class::MacroNonTerminal
                        } else if self.lexer.peek().tok == token::Not {
                            self.in_macro = true;
                            Class::Macro
                        } else {
                            Class::Ident
                        }
                    }
                }
            }

            token::Lifetime(..) => Class::Lifetime,

            token::Underscore | token::Eof | token::Interpolated(..) |
            token::MatchNt(..) | token::SubstNt(..) | token::Tilde | token::At => Class::None,
        };

        // Anything that didn't return above is the simple case where we the
        // class just spans a single token, so we can use the `string` method.
        out.string(Escape(&self.snip(tas.sp)), klass, Some(&tas))
    }
