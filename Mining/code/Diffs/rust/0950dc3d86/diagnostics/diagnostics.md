File_Code/rust/0950dc3d86/diagnostics/diagnostics_after.rs --- Rust
1254 E0272: r##"                                                                                                                                             1254 E0272: r##"
1255 The `#[rustc_on_unimplemented]` attribute lets you specify a custom error                                                                               1255 The `#[rustc_on_unimplemented]` attribute lets you specify a custom error
1256 message for when a particular trait isn't implemented on a type placed in a                                                                             1256 message for when a particular trait isn't implemented on a type placed in a
1257 position that needs that trait. For example, when the following code is                                                                                 1257 position that needs that trait. For example, when the following code is
1258 compiled:                                                                                                                                               1258 compiled:
1259                                                                                                                                                         1259 
1260 ```compile_fail                                                                                                                                         1260 ```compile_fail
1261 fn foo<T: Index<u8>>(x: T){}                                                                                                                            1261 fn foo<T: Index<u8>>(x: T){}
1262                                                                                                                                                         1262 
1263 #[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]                                                                            1263 #[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]
1264 trait Index<Idx> { ... }                                                                                                                                1264 trait Index<Idx> { /* ... */ }
1265                                                                                                                                                         1265 
1266 foo(true); // `bool` does not implement `Index<u8>`                                                                                                     1266 foo(true); // `bool` does not implement `Index<u8>`
1267 ```                                                                                                                                                     1267 ```
1268                                                                                                                                                         1268 
1269 There will be an error about `bool` not implementing `Index<u8>`, followed by a                                                                         1269 There will be an error about `bool` not implementing `Index<u8>`, followed by a
1270 note saying "the type `bool` cannot be indexed by `u8`".                                                                                                1270 note saying "the type `bool` cannot be indexed by `u8`".
1271                                                                                                                                                         1271 
1272 As you can see, you can specify type parameters in curly braces for                                                                                     1272 As you can see, you can specify type parameters in curly braces for
1273 substitution with the actual types (using the regular format string syntax) in                                                                          1273 substitution with the actual types (using the regular format string syntax) in
1274 a given situation. Furthermore, `{Self}` will substitute to the type (in this                                                                           1274 a given situation. Furthermore, `{Self}` will substitute to the type (in this
1275 case, `bool`) that we tried to use.                                                                                                                     1275 case, `bool`) that we tried to use.
1276                                                                                                                                                         1276 
1277 This error appears when the curly braces contain an identifier which doesn't                                                                            1277 This error appears when the curly braces contain an identifier which doesn't
1278 match with any of the type parameters or the string `Self`. This might happen                                                                           1278 match with any of the type parameters or the string `Self`. This might happen
1279 if you misspelled a type parameter, or if you intended to use literal curly                                                                             1279 if you misspelled a type parameter, or if you intended to use literal curly
1280 braces. If it is the latter, escape the curly braces with a second curly brace                                                                          1280 braces. If it is the latter, escape the curly braces with a second curly brace
1281 of the same type; e.g. a literal `{` is `{{`.                                                                                                           1281 of the same type; e.g. a literal `{` is `{{`.
1282 "##,                                                                                                                                                    1282 "##,
1283                                                                                                                                                         1283 
1284 E0273: r##"                                                                                                                                             1284 E0273: r##"
1285 The `#[rustc_on_unimplemented]` attribute lets you specify a custom error                                                                               1285 The `#[rustc_on_unimplemented]` attribute lets you specify a custom error
1286 message for when a particular trait isn't implemented on a type placed in a                                                                             1286 message for when a particular trait isn't implemented on a type placed in a
1287 position that needs that trait. For example, when the following code is                                                                                 1287 position that needs that trait. For example, when the following code is
1288 compiled:                                                                                                                                               1288 compiled:
1289                                                                                                                                                         1289 
1290 ```compile_fail                                                                                                                                         1290 ```compile_fail
1291 fn foo<T: Index<u8>>(x: T){}                                                                                                                            1291 fn foo<T: Index<u8>>(x: T){}
1292                                                                                                                                                         1292 
1293 #[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]                                                                            1293 #[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]
1294 trait Index<Idx> { ... }                                                                                                                                1294 trait Index<Idx> { /* ... */ }
1295                                                                                                                                                         1295 
1296 foo(true); // `bool` does not implement `Index<u8>`                                                                                                     1296 foo(true); // `bool` does not implement `Index<u8>`
1297 ```                                                                                                                                                     1297 ```
1298                                                                                                                                                         1298 
1299 there will be an error about `bool` not implementing `Index<u8>`, followed by a                                                                         1299 there will be an error about `bool` not implementing `Index<u8>`, followed by a
1300 note saying "the type `bool` cannot be indexed by `u8`".                                                                                                1300 note saying "the type `bool` cannot be indexed by `u8`".
1301                                                                                                                                                         1301 
1302 As you can see, you can specify type parameters in curly braces for                                                                                     1302 As you can see, you can specify type parameters in curly braces for
1303 substitution with the actual types (using the regular format string syntax) in                                                                          1303 substitution with the actual types (using the regular format string syntax) in
1304 a given situation. Furthermore, `{Self}` will substitute to the type (in this                                                                           1304 a given situation. Furthermore, `{Self}` will substitute to the type (in this
1305 case, `bool`) that we tried to use.                                                                                                                     1305 case, `bool`) that we tried to use.
1306                                                                                                                                                         1306 
1307 This error appears when the curly braces do not contain an identifier. Please                                                                           1307 This error appears when the curly braces do not contain an identifier. Please
1308 add one of the same name as a type parameter. If you intended to use literal                                                                            1308 add one of the same name as a type parameter. If you intended to use literal
1309 braces, use `{{` and `}}` to escape them.                                                                                                               1309 braces, use `{{` and `}}` to escape them.
1310 "##,                                                                                                                                                    1310 "##,
1311                                                                                                                                                         1311 
1312 E0274: r##"                                                                                                                                             1312 E0274: r##"
1313 The `#[rustc_on_unimplemented]` attribute lets you specify a custom error                                                                               1313 The `#[rustc_on_unimplemented]` attribute lets you specify a custom error
1314 message for when a particular trait isn't implemented on a type placed in a                                                                             1314 message for when a particular trait isn't implemented on a type placed in a
1315 position that needs that trait. For example, when the following code is                                                                                 1315 position that needs that trait. For example, when the following code is
1316 compiled:                                                                                                                                               1316 compiled:
1317                                                                                                                                                         1317 
1318 ```compile_fail                                                                                                                                         1318 ```compile_fail
1319 fn foo<T: Index<u8>>(x: T){}                                                                                                                            1319 fn foo<T: Index<u8>>(x: T){}
1320                                                                                                                                                         1320 
1321 #[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]                                                                            1321 #[rustc_on_unimplemented = "the type `{Self}` cannot be indexed by `{Idx}`"]
1322 trait Index<Idx> { ... }                                                                                                                                1322 trait Index<Idx> { /* ... */ }
1323                                                                                                                                                         1323 
1324 foo(true); // `bool` does not implement `Index<u8>`                                                                                                     1324 foo(true); // `bool` does not implement `Index<u8>`
1325 ```                                                                                                                                                     1325 ```
1326                                                                                                                                                         1326 
1327 there will be an error about `bool` not implementing `Index<u8>`, followed by a                                                                         1327 there will be an error about `bool` not implementing `Index<u8>`, followed by a
1328 note saying "the type `bool` cannot be indexed by `u8`".                                                                                                1328 note saying "the type `bool` cannot be indexed by `u8`".
1329                                                                                                                                                         1329 
1330 For this to work, some note must be specified. An empty attribute will not do                                                                           1330 For this to work, some note must be specified. An empty attribute will not do
1331 anything, please remove the attribute or add some helpful note for users of the                                                                         1331 anything, please remove the attribute or add some helpful note for users of the
1332 trait.                                                                                                                                                  1332 trait.
1333 "##,                                                                                                                                                    1333 "##,

