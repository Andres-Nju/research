File_Code/rust/e9c42257d5/dump_visitor/dump_visitor_after.rs --- Rust
1035                                            .sub_span_of_token(path.span, token::BinOp(token::Star));                                                    1035                                            .sub_span_of_token(item.span, token::BinOp(token::Star));
1036                         if !self.span.filter_generated(sub_span, path.span) {                                                                           1036                         if !self.span.filter_generated(sub_span, item.span) {

