    fn test_cfg_if_main() {
        // from https://github.com/rust-lang/rust/blob/3d211248393686e0f73851fc7548f6605220fbe1/src/libpanic_unwind/macros.rs#L9
        let rules = create_rules(
            r#"
        macro_rules! cfg_if {
            ($(
                if #[cfg($($meta:meta),*)] { $($it:item)* }
            ) else * else {
                $($it2:item)*
            }) => {
                __cfg_if_items! {
                    () ;
                    $( ( ($($meta),*) ($($it)*) ), )*
                    ( () ($($it2)*) ),
                }
            }
        }
"#,
        );

        assert_expansion(&rules, r#"
cfg_if !   { 
     if   # [ cfg ( target_env   =   "msvc" ) ]   { 
         // no extra unwinder support needed 
     }   else   if   # [ cfg ( all ( target_arch   =   "wasm32" ,   not ( target_os   =   "emscripten" ) ) ) ]   { 
         // no unwinder on the system! 
     }   else   { 
         mod   libunwind ; 
         pub   use   libunwind :: * ; 
     } 
 }        
"#,         
        "__cfg_if_items ! {() ; ((target_env = \"msvc\") ()) , ((all (target_arch = \"wasm32\" , not (target_os = \"emscripten\"))) ()) , (() (mod libunwind ; pub use libunwind :: * ;)) ,}");
    }
