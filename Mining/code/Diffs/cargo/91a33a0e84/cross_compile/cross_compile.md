File_Code/cargo/91a33a0e84/cross_compile/cross_compile_after.rs --- 1/2 --- Rust
215             r#"                                                                                                                                          215             r#"
216             #![feature(plugin_registrar, rustc_private)]                                                                                                 216             #![feature(plugin_registrar, rustc_private)]
217                                                                                                                                                          217 
218             extern crate rustc_plugin;                                                                                                                   218             extern crate rustc_plugin;
219             extern crate syntax;                                                                                                                         219             extern crate syntax;
220                                                                                                                                                          220 
221             use rustc_plugin::Registry;                                                                                                                  221             use rustc_plugin::Registry;
222             use syntax::tokenstream::TokenTree;                                                                                                          222             use syntax::tokenstream::TokenTree;
223             use syntax::codemap::Span;                                                                                                                   223             use syntax::source_map::Span;
224             use syntax::ast::*;                                                                                                                          224             use syntax::ast::*;
225             use syntax::ext::base::{ExtCtxt, MacEager, MacResult};                                                                                       225             use syntax::ext::base::{ExtCtxt, MacEager, MacResult};
226             use syntax::ext::build::AstBuilder;                                                                                                          226             use syntax::ext::build::AstBuilder;
227                                                                                                                                                          227 
228             #[plugin_registrar]                                                                                                                          228             #[plugin_registrar]
229             pub fn foo(reg: &mut Registry) {                                                                                                             229             pub fn foo(reg: &mut Registry) {
230                 reg.register_macro("bar", expand_bar);                                                                                                   230                 reg.register_macro("bar", expand_bar);
231             }                                                                                                                                            231             }
232                                                                                                                                                          232 
233             fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree])                                                                                 233             fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree])
234                           -> Box<MacResult + 'static> {                                                                                                  234                           -> Box<MacResult + 'static> {
235                 MacEager::expr(cx.expr_lit(sp, LitKind::Int(1, LitIntType::Unsuffixed)))                                                                 235                 MacEager::expr(cx.expr_lit(sp, LitKind::Int(1, LitIntType::Unsuffixed)))
236             }                                                                                                                                            236             }
237         "#,                                                                                                                                              237         "#,

File_Code/cargo/91a33a0e84/cross_compile/cross_compile_after.rs --- 2/2 --- Rust
314             r#"                                                                                                                                          314             r#"
315             #![feature(plugin_registrar, rustc_private)]                                                                                                 315             #![feature(plugin_registrar, rustc_private)]
316                                                                                                                                                          316 
317             extern crate rustc_plugin;                                                                                                                   317             extern crate rustc_plugin;
318             extern crate syntax;                                                                                                                         318             extern crate syntax;
319             extern crate baz;                                                                                                                            319             extern crate baz;
320                                                                                                                                                          320 
321             use rustc_plugin::Registry;                                                                                                                  321             use rustc_plugin::Registry;
322             use syntax::tokenstream::TokenTree;                                                                                                          322             use syntax::tokenstream::TokenTree;
323             use syntax::codemap::Span;                                                                                                                   323             use syntax::source_map::Span;
324             use syntax::ast::*;                                                                                                                          324             use syntax::ast::*;
325             use syntax::ext::base::{ExtCtxt, MacEager, MacResult};                                                                                       325             use syntax::ext::base::{ExtCtxt, MacEager, MacResult};
326             use syntax::ext::build::AstBuilder;                                                                                                          326             use syntax::ext::build::AstBuilder;
327             use syntax::ptr::P;                                                                                                                          327             use syntax::ptr::P;
328                                                                                                                                                          328 
329             #[plugin_registrar]                                                                                                                          329             #[plugin_registrar]
330             pub fn foo(reg: &mut Registry) {                                                                                                             330             pub fn foo(reg: &mut Registry) {
331                 reg.register_macro("bar", expand_bar);                                                                                                   331                 reg.register_macro("bar", expand_bar);
332             }                                                                                                                                            332             }
333                                                                                                                                                          333 
334             fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree])                                                                                 334             fn expand_bar(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree])
335                           -> Box<MacResult + 'static> {                                                                                                  335                           -> Box<MacResult + 'static> {
336                 let bar = Ident::from_str("baz");                                                                                                        336                 let bar = Ident::from_str("baz");
337                 let path = cx.path(sp, vec![bar.clone(), bar]);                                                                                          337                 let path = cx.path(sp, vec![bar.clone(), bar]);
338                 MacEager::expr(cx.expr_call(sp, cx.expr_path(path), vec![]))                                                                             338                 MacEager::expr(cx.expr_call(sp, cx.expr_path(path), vec![]))
339             }                                                                                                                                            339             }
340         "#,                                                                                                                                              340         "#,

