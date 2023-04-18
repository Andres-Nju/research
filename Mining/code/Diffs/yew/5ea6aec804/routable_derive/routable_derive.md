File_Code/yew/5ea6aec804/routable_derive/routable_derive_after.rs --- Rust
122                     quote! { Self::#ident { #(#fields: params.get(stringify!(#fields))?.parse().ok()?)*, } }                                             122                     quote! { Self::#ident { #(#fields: params.get(stringify!(#fields))?.parse().ok()?,)* } }

