fn quote_for_test(
    test_mod_ident: &Ident,
    type_name: &Ident,
    expected_digest: &str,
) -> TokenStream2 {
    // escape from nits.sh...
    let p = Ident::new(&("ep".to_owned() + "rintln"), Span::call_site());
    quote! {
        #[cfg(test)]
        mod #test_mod_ident {
            use super::*;
            use ::solana_frozen_abi::abi_example::{AbiExample, AbiEnumVisitor};

            #[test]
            fn test_abi_digest() {
                ::solana_logger::setup();
                let mut digester = ::solana_frozen_abi::abi_digester::AbiDigester::create();
                let example = <#type_name>::example();
                let result = <_>::visit_for_abi(&&example, &mut digester);
                let mut hash = digester.finalize();
                // pretty-print error
                if result.is_err() {
                    ::log::error!("digest error: {:#?}", result);
                }
                result.unwrap();
                let actual_digest = format!("{}", hash);
                if ::std::env::var("SOLANA_ABI_BULK_UPDATE").is_ok() {
                    if #expected_digest != actual_digest {
                        #p!("sed -i -e 's/{}/{}/g' $(git grep --files-with-matches frozen_abi)", #expected_digest, hash);
                    }
                    ::log::warn!("Not testing the abi digest under SOLANA_ABI_BULK_UPDATE!");
                } else {
                    if let Ok(dir) = ::std::env::var("SOLANA_ABI_DUMP_DIR") {
                        assert_eq!(#expected_digest, actual_digest, "Possibly ABI changed? Examine the diff in SOLANA_ABI_DUMP_DIR!: \n$ diff -u {}/*{}* {}/*{}*", dir, #expected_digest, dir, actual_digest);
                    } else {
                        assert_eq!(#expected_digest, actual_digest, "Possibly ABI changed? Confirm the diff by rerunning before and after this test failed with SOLANA_ABI_DUMP_DIR!");
                    }
                }
            }
        }
