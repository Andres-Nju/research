File_Code/cargo/803b9cd8ae/cross_compile/cross_compile_after.rs --- 1/2 --- Rust
201             r#"                                                                                                                                          201             r#"
202             #![feature(plugin_registrar, rustc_private)]                                                                                                 202             #![feature(plugin_registrar, rustc_private)]
203                                                                                                                                                          203 
204             extern crate rustc_driver;                                                                                                                   204             extern crate rustc_driver;
205             extern crate syntax;                                                                                                                         205             extern crate syntax;
206                                                                                                                                                          206 
207             use rustc_driver::plugin::Registry;                                                                                                          207             use rustc_driver::plugin::Registry;
208             use syntax::tokenstream::TokenTree;                                                                                                          208             use syntax::tokenstream::TokenStream;
209             use syntax::source_map::Span;                                                                                                                209             use syntax::source_map::Span;
210             use syntax::ast::*;                                                                                                                          210             use syntax::ast::*;
211             use syntax::ext::base::{ExtCtxt, MacEager, MacResult};                                                                                       211             use syntax::ext::base::{ExtCtxt, MacEager, MacResult};
212                                                                                                                                                          212 
213             #[plugin_registrar]                                                                                                                          213             #[plugin_registrar]
214             pub fn foo(reg: &mut Registry) {                                                                                                             214             pub fn foo(reg: &mut Registry) {
215                 reg.register_macro("bar", expand_bar);                                                                                                   215                 reg.register_macro("bar", expand_bar);
216             }                                                                                                                                            216             }
217                                                                                                                                                          217 
218             fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree])                                                                                 218             fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: TokenStream)
219                           -> Box<MacResult + 'static> {                                                                                                  219                           -> Box<MacResult + 'static> {
220                 MacEager::expr(cx.expr_lit(sp, LitKind::Int(1, LitIntType::Unsuffixed)))                                                                 220                 MacEager::expr(cx.expr_lit(sp, LitKind::Int(1, LitIntType::Unsuffixed)))
221             }                                                                                                                                            221             }
222         "#,                                                                                                                                              222         "#,

File_Code/cargo/803b9cd8ae/cross_compile/cross_compile_after.rs --- 2/2 --- Rust
296             r#"                                                                                                                                          296             r#"
297             #![feature(plugin_registrar, rustc_private)]                                                                                                 297             #![feature(plugin_registrar, rustc_private)]
298                                                                                                                                                          298 
299             extern crate rustc_driver;                                                                                                                   299             extern crate rustc_driver;
300             extern crate syntax;                                                                                                                         300             extern crate syntax;
301             extern crate baz;                                                                                                                            301             extern crate baz;
302                                                                                                                                                          302 
303             use rustc_driver::plugin::Registry;                                                                                                          303             use rustc_driver::plugin::Registry;
304             use syntax::tokenstream::TokenTree;                                                                                                          304             use syntax::tokenstream::TokenStream;
305             use syntax::source_map::Span;                                                                                                                305             use syntax::source_map::Span;
306             use syntax::ast::*;                                                                                                                          306             use syntax::ast::*;
307             use syntax::ext::base::{ExtCtxt, MacEager, MacResult};                                                                                       307             use syntax::ext::base::{ExtCtxt, MacEager, MacResult};
308             use syntax::ptr::P;                                                                                                                          308             use syntax::ptr::P;
309                                                                                                                                                          309 
310             #[plugin_registrar]                                                                                                                          310             #[plugin_registrar]
311             pub fn foo(reg: &mut Registry) {                                                                                                             311             pub fn foo(reg: &mut Registry) {
312                 reg.register_macro("bar", expand_bar);                                                                                                   312                 reg.register_macro("bar", expand_bar);
313             }                                                                                                                                            313             }
314                                                                                                                                                          314 
315             fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree])                                                                                 315             fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: TokenStream)
316                           -> Box<MacResult + 'static> {                                                                                                  316                           -> Box<MacResult + 'static> {
317                 let bar = Ident::from_str("baz");                                                                                                        317                 let bar = Ident::from_str("baz");
318                 let path = cx.path(sp, vec![bar.clone(), bar]);                                                                                          318                 let path = cx.path(sp, vec![bar.clone(), bar]);
319                 MacEager::expr(cx.expr_call(sp, cx.expr_path(path), vec![]))                                                                             319                 MacEager::expr(cx.expr_call(sp, cx.expr_path(path), vec![]))
320             }                                                                                                                                            320             }
321         "#,                                                                                                                                              321         "#,

