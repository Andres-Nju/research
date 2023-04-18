File_Code/rust/bd25507f0e/build/build_after.rs --- Rust
27     all.push_str(r###"                                                                                                                                    27     all.push_str(r###"
28 fn register_all() -> Vec<(&'static str, Option<&'static str>)> {                                                                                          28 fn register_all() -> Vec<(&'static str, Option<&'static str>)> {
29     let mut long_codes: Vec<(&'static str, Option<&'static str>)> = Vec::new();                                                                           29     let mut long_codes: Vec<(&'static str, Option<&'static str>)> = Vec::new();
30     macro_rules! register_diagnostics {                                                                                                                   30     macro_rules! register_diagnostics {
31         ($($ecode:ident: $message:expr,)*) => (                                                                                                           31         ($($ecode:ident: $message:expr,)*) => (
32             register_diagnostics!{$($ecode:$message,)* ;}                                                                                                 32             register_diagnostics!{$($ecode:$message,)* ;}
33         );                                                                                                                                                33         );
34                                                                                                                                                           34 
35         ($($ecode:ident: $message:expr,)* ; $($code:ident,)*) => (                                                                                        35         ($($ecode:ident: $message:expr,)* ; $($code:ident,)*) => (
36             $(                                                                                                                                            36             $(
37                 {long_codes.extend([                                                                                                                      37                 {long_codes.extend([
38                     (stringify!($ecode), Some(stringify!($message))),                                                                                     38                     (stringify!($ecode), Some($message)),
39                 ].iter());}                                                                                                                               39                 ].iter());}
40             )*                                                                                                                                            40             )*
41             $(                                                                                                                                            41             $(
42                 {long_codes.extend([                                                                                                                      42                 {long_codes.extend([
43                     stringify!($code),                                                                                                                    43                     stringify!($code),
44                 ].iter().cloned().map(|s| (s, None)).collect::<Vec<_>>());}                                                                               44                 ].iter().cloned().map(|s| (s, None)).collect::<Vec<_>>());}
45             )*                                                                                                                                            45             )*
46         )                                                                                                                                                 46         )
47     }                                                                                                                                                     47     }
48 "###);                                                                                                                                                    48 "###);

