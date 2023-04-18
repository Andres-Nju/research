    fn build_from_path(&self) -> TokenStream {
        let from_path_matches = self.variants.iter().enumerate().map(|(i, variant)| {
            let ident = &variant.ident;
            let right = match &variant.fields {
                Fields::Unit => quote! { Self::#ident },
                Fields::Named(field) => {
                    let fields = field.named.iter().map(|it| {
                        //named fields have idents
                        it.ident.as_ref().unwrap()
                    });
                    quote! { Self::#ident { #(#fields: params.get(stringify!(#fields))?.parse().ok()?,)* } }
                }
                Fields::Unnamed(_) => unreachable!(), // already checked
            };

            let left = self.ats.get(i).unwrap();
            quote! {
                #left => ::std::option::Option::Some(#right)
            }
        });

        quote! {
            fn from_path(path: &str, params: &::std::collections::HashMap<&str, &str>) -> ::std::option::Option<Self> {
                match path {
                    #(#from_path_matches),*,
                    _ => ::std::option::Option::None,
                }
            }
        }
    }

    fn build_to_path(&self) -> TokenStream {
        let to_path_matches = self.variants.iter().enumerate().map(|(i, variant)| {
            let ident = &variant.ident;
            let mut right = self.ats.get(i).unwrap().value();

            match &variant.fields {
                Fields::Unit => quote! { Self::#ident => ::std::string::ToString::to_string(#right) },
                Fields::Named(field) => {
                    let fields = field
                        .named
                        .iter()
                        .map(|it| it.ident.as_ref().unwrap())
                        .collect::<Vec<_>>();

                    for field in fields.iter() {
                        // :param -> {param}
                        // so we can pass it to `format!("...", param)`
                        right = right.replace(&format!(":{}", field), &format!("{{{}}}", field))
                    }

                    quote! {
                        Self::#ident { #(#fields),* } => ::std::format!(#right, #(#fields = #fields),*)
                    }
                }
                Fields::Unnamed(_) => unreachable!(), // already checked
            }
        });

        quote! {
            fn to_path(&self) -> ::std::string::String {
                match self {
                    #(#to_path_matches),*,
                }
            }
        }
    }
}

pub fn routable_derive_impl(input: Routable) -> TokenStream {
    let Routable {
        ats,
        not_found_route,
        ident,
        ..
    } = &input;

    let from_path = input.build_from_path();
    let to_path = input.build_to_path();

    let not_found_route = match not_found_route {
        Some(route) => quote! { ::std::option::Option::Some(Self::#route) },
        None => quote! { ::std::option::Option::None },
    };

    let cache_thread_local_ident = Ident::new(
        &format!("__{}_ROUTER_CURRENT_ROUTE_CACHE", ident),
        ident.span(),
    );

    quote! {
        ::std::thread_local! {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            static #cache_thread_local_ident: ::std::cell::RefCell<::std::option::Option<#ident>> = ::std::cell::RefCell::new(::std::option::Option::None);
        }

        #[automatically_derived]
        impl ::yew_router::Routable for #ident {
            #from_path
            #to_path

            fn routes() -> ::std::vec::Vec<&'static str> {
                ::std::vec![#(#ats),*]
            }

            fn not_found_route() -> ::std::option::Option<Self> {
                #not_found_route
            }

            fn current_route() -> ::std::option::Option<Self> {
                #cache_thread_local_ident.with(|val| ::std::clone::Clone::clone(&*val.borrow()))
            }

            fn recognize(pathname: &str) -> ::std::option::Option<Self> {
                ::std::thread_local! {
                    static ROUTER: ::yew_router::__macro::Router = ::yew_router::__macro::build_router::<#ident>();
                }
                let route = ROUTER.with(|router| ::yew_router::__macro::recognize_with_router(router, pathname));
                {
                    let route = ::std::clone::Clone::clone(&route);
                    #cache_thread_local_ident.with(move |val| {
                        *val.borrow_mut() = route;
                    });
                }
                route
            }

            fn cleanup() {
                #cache_thread_local_ident.with(move |val| {
                    *val.borrow_mut() = ::std::option::Option::None;
                });
            }
        }
    }
}
