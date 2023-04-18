pub fn derive_string_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse::<syn::DeriveInput>(input)
        .map(From::from)
        .expect("failed to parse derive input");
    let mut tts = TokenStream::new();

    make_as_str(&input).to_tokens(&mut tts);
    make_from_str(&input).to_tokens(&mut tts);

    make_serialize(&input).to_tokens(&mut tts);
    make_deserialize(&input).to_tokens(&mut tts);

    derive_fmt(&input, quote_spanned!(Span::call_site() => std::fmt::Debug)).to_tokens(&mut tts);
    derive_fmt(
        &input,
        quote_spanned!(Span::call_site() => std::fmt::Display),
    )
    .to_tokens(&mut tts);

    print("derive(StringEnum)", tts)
}
