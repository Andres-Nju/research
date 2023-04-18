File_Code/swc/97945d02b8/lib/lib_after.rs --- Rust
78     derive_fmt(&input, quote_spanned!(call_site() => std::fmt::Debug)).to_tokens(&mut tts);                                                               78     derive_fmt(&input, quote_spanned!(Span::call_site() => std::fmt::Debug)).to_tokens(&mut tts);
79     derive_fmt(&input, quote_spanned!(call_site() => std::fmt::Display)).to_tokens(&mut tts);                                                             79     derive_fmt(
                                                                                                                                                             80         &input,
                                                                                                                                                             81         quote_spanned!(Span::call_site() => std::fmt::Display),

