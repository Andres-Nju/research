fn structure(ast: syn::DeriveInput, ty_compile: quote::Tokens, ty_run: quote::Tokens)
             -> quote::Tokens {
    let name = &ast.ident;
    let fields = match ast.body {
        syn::Body::Struct(syn::VariantData::Struct(ref fields)) => fields,
        _ => panic!("gfx-rs custom derives can only be casted on structs"),
    };
    let match_name = fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        quote! {
            stringify!(#ident) => Some(Element {
                format: <#ty as #ty_compile>::get_format(),
                offset: ((&tmp.#ident as *const _ as usize) - base) as ElemOffset + big_offset,
            }),
        }
    });
