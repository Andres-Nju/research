            fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
                match *self {
                    ContentItem::String(ref s) => {
                        cssparser::serialize_string(&**s, dest)
                    }
                    ContentItem::Counter(ref s, ref list_style_type) => {
                        try!(dest.write_str("counter("));
                        try!(cssparser::serialize_identifier(&**s, dest));
                        try!(dest.write_str(", "));
                        try!(list_style_type.to_css(dest));
                        dest.write_str(")")
                    }
                    ContentItem::Counters(ref s, ref separator, ref list_style_type) => {
                        try!(dest.write_str("counters("));
                        try!(cssparser::serialize_identifier(&**s, dest));
                        try!(dest.write_str(", "));
                        try!(cssparser::serialize_string(&**separator, dest));
                        try!(dest.write_str(", "));
                        try!(list_style_type.to_css(dest));
                        dest.write_str(")")
                    }
                    ContentItem::OpenQuote => dest.write_str("open-quote"),
                    ContentItem::CloseQuote => dest.write_str("close-quote"),
                    ContentItem::NoOpenQuote => dest.write_str("no-open-quote"),
                    ContentItem::NoCloseQuote => dest.write_str("no-close-quote"),

                    % if product == "gecko":
                        ContentItem::MozAltContent => dest.write_str("-moz-alt-content"),
                        ContentItem::Attr(ref ns, ref attr) => {
                            dest.write_str("attr(")?;
                            if let Some(ref ns) = *ns {
                                cssparser::Token::Ident((&**ns).into()).to_css(dest)?;
                                dest.write_str("|")?;
                            }
                            cssparser::Token::Ident((&**attr).into()).to_css(dest)?;
                            dest.write_str(")")
                        }
                        ContentItem::Url(ref url) => url.to_css(dest),
                    % endif
                }
            }
