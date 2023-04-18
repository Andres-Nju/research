pub(super) fn prepare_vars(
    src: &dyn ToCode,
    vars: Punctuated<QuoteVar, Token![,]>,
) -> (Vec<syn::Stmt>, AHashMap<VarPos, Vars>) {
    let mut stmts = vec![];
    let mut init_map = AHashMap::<_, Vars>::default();

    for var in vars {
        let value = var.value;

        let ident = var.name.clone();
        let ident_str = ident.to_string();

        let pos = match var.ty {
            Some(syn::Type::Path(syn::TypePath {
                qself: None,
                path:
                    syn::Path {
                        leading_colon: None,
                        segments,
                    },
            })) => {
                let segment = segments.first().unwrap();
                match segment.ident.to_string().as_str() {
                    "Ident" => VarPos::Ident,
                    "Expr" => VarPos::Expr,
                    "Pat" => VarPos::Pat,
                    "Str" => VarPos::Str,
                    _ => panic!("Invalid type: {}", segment.ident),
                }
            }
            None => VarPos::Ident,
            _ => {
                panic!(
                    "Var type should be one of: Ident, Expr, Pat; got {:?}",
                    var.ty
                )
            }
        };

        let var_ident = syn::Ident::new(&format!("quote_var_{}", ident), ident.span());

        let old = init_map.entry(pos).or_default().insert(
            ident_str.clone(),
            VarData {
                pos,
                is_counting: true,
                clone: Default::default(),
                ident: var_ident.clone(),
            },
        );

        if let Some(old) = old {
            panic!("Duplicate variable name: {}", ident_str);
        }

        let type_name = Ident::new(
            match pos {
                VarPos::Ident => "Ident",
                VarPos::Expr => "Expr",
                VarPos::Pat => "Pat",
                VarPos::Str => "Str",
            },
            call_site(),
        );
        stmts.push(parse_quote! {
            let #var_ident: swc_core::ecma::ast::#type_name = #value;
        });
    }

    // Use `ToCode` to count how many times each variable is used.
    let mut cx = Ctx { vars: init_map };

    src.to_code(&cx);

    // We are done
    cx.vars
        .iter_mut()
        .for_each(|(k, v)| v.iter_mut().for_each(|(_, v)| v.is_counting = false));

    (stmts, cx.vars)
}
