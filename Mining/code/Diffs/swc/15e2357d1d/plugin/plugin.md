File_Code/swc/15e2357d1d/plugin/plugin_after.rs --- Rust
234             r##"use swc_core::{                                                                                                                          234             r##"use swc_core::{
235     ast::Program,                                                                                                                                        235     ast::Program,
236     plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},                                                                                 236     plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
237     testing_transform::test,                                                                                                                             237     testing_transform::test,
238     visit::{as_folder, FoldWith, VisitMut},                                                                                                              238     visit::{as_folder, FoldWith, VisitMut},
239 };                                                                                                                                                       239 };
240                                                                                                                                                          240 
241 pub struct TransformVisitor;                                                                                                                             241 pub struct TransformVisitor;
242                                                                                                                                                          242 
243 impl VisitMut for TransformVisitor {                                                                                                                     243 impl VisitMut for TransformVisitor {
244     // Implement necessary visit_mut_* methods for actual custom transform.                                                                              244     // Implement necessary visit_mut_* methods for actual custom transform.
245     // A comprehensive list of possible visitor methods can be found here:                                                                               245     // A comprehensive list of possible visitor methods can be found here:
246     // https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html                                                                                         246     // https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html
247 }                                                                                                                                                        247 }
248                                                                                                                                                          248 
249 /// An example plugin function with macro support.                                                                                                       249 /// An example plugin function with macro support.
250 /// `plugin_transform` macro interop pointers into deserialized structs, as well                                                                         250 /// `plugin_transform` macro interop pointers into deserialized structs, as well
251 /// as returning ptr back to host.                                                                                                                       251 /// as returning ptr back to host.
252 ///                                                                                                                                                      252 ///
253 /// It is possible to opt out from macro by writing transform fn manually                                                                                253 /// It is possible to opt out from macro by writing transform fn manually
254 /// if plugin need to handle low-level ptr directly via                                                                                                  254 /// if plugin need to handle low-level ptr directly via
255 /// `__transform_plugin_process_impl(                                                                                                                    255 /// `__transform_plugin_process_impl(
256 ///     ast_ptr: *const u8, ast_ptr_len: i32,                                                                                                            256 ///     ast_ptr: *const u8, ast_ptr_len: i32,
257 ///     unresolved_mark: u32, should_enable_comments_proxy: i32) ->                                                                                      257 ///     unresolved_mark: u32, should_enable_comments_proxy: i32) ->
258 ///     i32 /*  0 for success, fail otherwise.                                                                                                           258 ///     i32 /*  0 for success, fail otherwise.
259 ///             Note this is only for internal pointer interop result,                                                                                   259 ///             Note this is only for internal pointer interop result,
260 ///             not actual transform result */`                                                                                                          260 ///             not actual transform result */`
261 ///                                                                                                                                                      261 ///
262 /// This requires manual handling of serialization / deserialization from ptrs.                                                                          262 /// This requires manual handling of serialization / deserialization from ptrs.
263 /// Refer swc_plugin_macro to see how does it work internally.                                                                                           263 /// Refer swc_plugin_macro to see how does it work internally.
264 #[plugin_transform]                                                                                                                                      264 #[plugin_transform]
265 pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {                                                       265 pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
266     program.fold_with(&mut as_folder(TransformVisitor))                                                                                                  266     program.fold_with(&mut as_folder(TransformVisitor))
267 }                                                                                                                                                        267 }
268                                                                                                                                                          268 
269 // An example to test plugin transform.                                                                                                                  269 // An example to test plugin transform.
270 // Recommended streategy to test plugin's transform is verify                                                                                            270 // Recommended strategy to test plugin's transform is verify
271 // the Visitor's behavior, instead of trying to run `process_transform` with mocks                                                                       271 // the Visitor's behavior, instead of trying to run `process_transform` with mocks
272 // unless explicitly required to do so.                                                                                                                  272 // unless explicitly required to do so.
273 test!(                                                                                                                                                   273 test!(
274     Default::default(),                                                                                                                                  274     Default::default(),
275     |_| as_folder(TransformVisitor),                                                                                                                     275     |_| as_folder(TransformVisitor),
276     boo,                                                                                                                                                 276     boo,
277     // Input codes                                                                                                                                       277     // Input codes
278     r#"console.log("transform");"#,                                                                                                                      278     r#"console.log("transform");"#,
279     // Output codes after transformed with plugin                                                                                                        279     // Output codes after transformed with plugin
280     r#"console.log("transform");"#                                                                                                                       280     r#"console.log("transform");"#
281 );"##                                                                                                                                                    281 );"##

