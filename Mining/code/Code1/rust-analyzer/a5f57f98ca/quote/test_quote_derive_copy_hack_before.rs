    fn test_quote_derive_copy_hack() {
        // Assume the given struct is:
        // struct Foo {
        //  name: String,
        //  id: u32,
        // }
        let struct_name = mk_ident("Foo");
        let fields = [mk_ident("name"), mk_ident("id")];
        let fields = fields
            .into_iter()
            .map(|it| quote!(#it: self.#it.clone(), ).token_trees.clone())
            .flatten();

        let list = tt::Subtree { delimiter: tt::Delimiter::Brace, token_trees: fields.collect() };

        let quoted = quote! {
            impl Clone for #struct_name {
                fn clone(&self) -> Self {
                    Self #list
                }
            }
        };
