File_Code/rust/c22cc1d67a/error_codes/error_codes_after.rs --- 1/2 --- Rust
2023 E0515: r##"                                                                                                                                             2023 E0515: r##"
2024 Cannot return value that references local variable                                                                                                      2024 Cannot return value that references local variable
2025                                                                                                                                                         2025 
2026 Local variables, function parameters and temporaries are all dropped before the                                                                         2026 Local variables, function parameters and temporaries are all dropped before the
2027 end of the function body. So a reference to them cannot be returned.                                                                                    2027 end of the function body. So a reference to them cannot be returned.
2028                                                                                                                                                         2028 
2029 ```compile_fail,E0515                                                                                                                                   2029 ```compile_fail,E0515
2030 #![feature(nll)]                                                                                                                                        2030 fn get_dangling_reference() -> &'static i32 {
2031 fn get_dangling_reference() -> &'static i32 {                                                                                                           2031     let x = 0;
2032     let x = 0;                                                                                                                                          2032     &x
2033     &x                                                                                                                                                  2033 }
2034 }                                                                                                                                                       2034 ```
2035 ```                                                                                                                                                     2035 
2036                                                                                                                                                         2036 ```compile_fail,E0515
2037 ```compile_fail,E0515                                                                                                                                   2037 use std::slice::Iter;
2038 #![feature(nll)]                                                                                                                                        2038 fn get_dangling_iterator<'a>() -> Iter<'a, i32> {
2039 use std::slice::Iter;                                                                                                                                   2039     let v = vec![1, 2, 3];
2040 fn get_dangling_iterator<'a>() -> Iter<'a, i32> {                                                                                                       2040     v.iter()
2041     let v = vec![1, 2, 3];                                                                                                                              2041 }
2042     v.iter()                                                                                                                                            2042 ```
2043 }                                                                                                                                                       2043 
2044 ```                                                                                                                                                     2044 Consider returning an owned value instead:
2045                                                                                                                                                         2045 
2046 Consider returning an owned value instead:                                                                                                              2046 ```
2047                                                                                                                                                         2047 use std::vec::IntoIter;
2048 ```                                                                                                                                                     2048 
2049 use std::vec::IntoIter;                                                                                                                                 2049 fn get_integer() -> i32 {
2050                                                                                                                                                         2050     let x = 0;
2051 fn get_integer() -> i32 {                                                                                                                               2051     x
2052     let x = 0;                                                                                                                                          2052 }
2053     x                                                                                                                                                   2053 
2054 }                                                                                                                                                       2054 fn get_owned_iterator() -> IntoIter<i32> {
2055                                                                                                                                                         2055     let v = vec![1, 2, 3];
2056 fn get_owned_iterator() -> IntoIter<i32> {                                                                                                              2056     v.into_iter()
2057     let v = vec![1, 2, 3];                                                                                                                              2057 }
2058     v.into_iter()                                                                                                                                       2058 ```
2059 }                                                                                                                                                       .... 
2060 ```                                                                                                                                                     .... 
2061 "##,                                                                                                                                                    2059 "##,

File_Code/rust/c22cc1d67a/error_codes/error_codes_after.rs --- 2/2 --- Rust
2229 E0712: r##"                                                                                                                                             2227 E0712: r##"
2230 This error occurs because a borrow of a thread-local variable was made inside a                                                                         2228 This error occurs because a borrow of a thread-local variable was made inside a
2231 function which outlived the lifetime of the function.                                                                                                   2229 function which outlived the lifetime of the function.
2232                                                                                                                                                         2230 
2233 Example of erroneous code:                                                                                                                              2231 Example of erroneous code:
2234                                                                                                                                                         2232 
2235 ```compile_fail,E0712                                                                                                                                   2233 ```compile_fail,E0712
2236 #![feature(nll)]                                                                                                                                        2234 #![feature(thread_local)]
2237 #![feature(thread_local)]                                                                                                                               2235 
2238                                                                                                                                                         2236 #[thread_local]
2239 #[thread_local]                                                                                                                                         2237 static FOO: u8 = 3;
2240 static FOO: u8 = 3;                                                                                                                                     2238 
2241                                                                                                                                                         2239 fn main() {
2242 fn main() {                                                                                                                                             2240     let a = &FOO; // error: thread-local variable borrowed past end of function
2243     let a = &FOO; // error: thread-local variable borrowed past end of function                                                                         2241 
2244                                                                                                                                                         2242     std::thread::spawn(move || {
2245     std::thread::spawn(move || {                                                                                                                        2243         println!("{}", a);
2246         println!("{}", a);                                                                                                                              2244     });
2247     });                                                                                                                                                 2245 }
2248 }                                                                                                                                                       2246 ```
2249 ```                                                                                                                                                     .... 
2250 "##,                                                                                                                                                    2247 "##,
2251                                                                                                                                                         2248 
2252 E0713: r##"                                                                                                                                             2249 E0713: r##"
2253 This error occurs when an attempt is made to borrow state past the end of the                                                                           2250 This error occurs when an attempt is made to borrow state past the end of the
2254 lifetime of a type that implements the `Drop` trait.                                                                                                    2251 lifetime of a type that implements the `Drop` trait.
2255                                                                                                                                                         2252 
2256 Example of erroneous code:                                                                                                                              2253 Example of erroneous code:
2257                                                                                                                                                         2254 
2258 ```compile_fail,E0713                                                                                                                                   2255 ```compile_fail,E0713
2259 #![feature(nll)]                                                                                                                                        2256 #![feature(nll)]
2260                                                                                                                                                         2257 
2261 pub struct S<'a> { data: &'a mut String }                                                                                                               2258 pub struct S<'a> { data: &'a mut String }
2262                                                                                                                                                         2259 
2263 impl<'a> Drop for S<'a> {                                                                                                                               2260 impl<'a> Drop for S<'a> {
2264     fn drop(&mut self) { self.data.push_str("being dropped"); }                                                                                         2261     fn drop(&mut self) { self.data.push_str("being dropped"); }
2265 }                                                                                                                                                       2262 }
2266                                                                                                                                                         2263 
2267 fn demo<'a>(s: S<'a>) -> &'a mut String { let p = &mut *s.data; p }                                                                                     2264 fn demo<'a>(s: S<'a>) -> &'a mut String { let p = &mut *s.data; p }
2268 ```                                                                                                                                                     2265 ```
2269                                                                                                                                                         2266 
2270 Here, `demo` tries to borrow the string data held within its                                                                                            2267 Here, `demo` tries to borrow the string data held within its
2271 argument `s` and then return that borrow. However, `S` is                                                                                               2268 argument `s` and then return that borrow. However, `S` is
2272 declared as implementing `Drop`.                                                                                                                        2269 declared as implementing `Drop`.
2273                                                                                                                                                         2270 
2274 Structs implementing the `Drop` trait have an implicit destructor that                                                                                  2271 Structs implementing the `Drop` trait have an implicit destructor that
2275 gets called when they go out of scope. This destructor gets exclusive                                                                                   2272 gets called when they go out of scope. This destructor gets exclusive
2276 access to the fields of the struct when it runs.                                                                                                        2273 access to the fields of the struct when it runs.
2277                                                                                                                                                         2274 
2278 This means that when `s` reaches the end of `demo`, its destructor                                                                                      2275 This means that when `s` reaches the end of `demo`, its destructor
2279 gets exclusive access to its `&mut`-borrowed string data.  allowing                                                                                     2276 gets exclusive access to its `&mut`-borrowed string data.  allowing
2280 another borrow of that string data (`p`), to exist across the drop of                                                                                   2277 another borrow of that string data (`p`), to exist across the drop of
2281 `s` would be a violation of the principle that `&mut`-borrows have                                                                                      2278 `s` would be a violation of the principle that `&mut`-borrows have
2282 exclusive, unaliased access to their referenced data.                                                                                                   2279 exclusive, unaliased access to their referenced data.
2283                                                                                                                                                         2280 
2284 This error can be fixed by changing `demo` so that the destructor does                                                                                  2281 This error can be fixed by changing `demo` so that the destructor does
2285 not run while the string-data is borrowed; for example by taking `S`                                                                                    2282 not run while the string-data is borrowed; for example by taking `S`
2286 by reference:                                                                                                                                           2283 by reference:
2287                                                                                                                                                         2284 
2288 ```                                                                                                                                                     2285 ```
2289 #![feature(nll)]                                                                                                                                        2286 pub struct S<'a> { data: &'a mut String }
2290                                                                                                                                                         2287 
2291 pub struct S<'a> { data: &'a mut String }                                                                                                               2288 impl<'a> Drop for S<'a> {
2292                                                                                                                                                         2289     fn drop(&mut self) { self.data.push_str("being dropped"); }
2293 impl<'a> Drop for S<'a> {                                                                                                                               2290 }
2294     fn drop(&mut self) { self.data.push_str("being dropped"); }                                                                                         2291 
2295 }                                                                                                                                                       2292 fn demo<'a>(s: &'a mut S<'a>) -> &'a mut String { let p = &mut *(*s).data; p }
2296                                                                                                                                                         2293 ```
2297 fn demo<'a>(s: &'a mut S<'a>) -> &'a mut String { let p = &mut *(*s).data; p }                                                                          2294 
2298 ```                                                                                                                                                     2295 Note that this approach needs a reference to S with lifetime `'a`.
2299                                                                                                                                                         2296 Nothing shorter than `'a` will suffice: a shorter lifetime would imply
2300 Note that this approach needs a reference to S with lifetime `'a`.                                                                                      2297 that after `demo` finishes executing, something else (such as the
2301 Nothing shorter than `'a` will suffice: a shorter lifetime would imply                                                                                  2298 destructor!) could access `s.data` after the end of that shorter
2302 that after `demo` finishes executing, something else (such as the                                                                                       2299 lifetime, which would again violate the `&mut`-borrow's exclusive
2303 destructor!) could access `s.data` after the end of that shorter                                                                                        2300 access.
2304 lifetime, which would again violate the `&mut`-borrow's exclusive                                                                                       .... 
2305 access.                                                                                                                                                 .... 
2306 "##,                                                                                                                                                    2301 "##,
2307                                                                                                                                                         2302 
2308 E0716: r##"                                                                                                                                             2303 E0716: r##"
2309 This error indicates that a temporary value is being dropped                                                                                            2304 This error indicates that a temporary value is being dropped
2310 while a borrow is still in active use.                                                                                                                  2305 while a borrow is still in active use.
2311                                                                                                                                                         2306 
2312 Erroneous code example:                                                                                                                                 2307 Erroneous code example:
2313                                                                                                                                                         2308 
2314 ```compile_fail,E0716                                                                                                                                   2309 ```compile_fail,E0716
2315 # #![feature(nll)]                                                                                                                                      2310 fn foo() -> i32 { 22 }
2316 fn foo() -> i32 { 22 }                                                                                                                                  2311 fn bar(x: &i32) -> &i32 { x }
2317 fn bar(x: &i32) -> &i32 { x }                                                                                                                           2312 let p = bar(&foo());
2318 let p = bar(&foo());                                                                                                                                    2313          // ------ creates a temporary
2319          // ------ creates a temporary                                                                                                                  2314 let q = *p;
2320 let q = *p;                                                                                                                                             2315 ```
2321 ```                                                                                                                                                     2316 
2322                                                                                                                                                         2317 Here, the expression `&foo()` is borrowing the expression
2323 Here, the expression `&foo()` is borrowing the expression                                                                                               2318 `foo()`. As `foo()` is a call to a function, and not the name of
2324 `foo()`. As `foo()` is a call to a function, and not the name of                                                                                        2319 a variable, this creates a **temporary** -- that temporary stores
2325 a variable, this creates a **temporary** -- that temporary stores                                                                                       2320 the return value from `foo()` so that it can be borrowed.
2326 the return value from `foo()` so that it can be borrowed.                                                                                               2321 You could imagine that `let p = bar(&foo());` is equivalent
2327 You could imagine that `let p = bar(&foo());` is equivalent                                                                                             2322 to this:
2328 to this:                                                                                                                                                2323 
2329                                                                                                                                                         2324 ```compile_fail,E0597
2330 ```compile_fail,E0597                                                                                                                                   2325 # fn foo() -> i32 { 22 }
2331 # fn foo() -> i32 { 22 }                                                                                                                                2326 # fn bar(x: &i32) -> &i32 { x }
2332 # fn bar(x: &i32) -> &i32 { x }                                                                                                                         2327 let p = {
2333 let p = {                                                                                                                                               2328   let tmp = foo(); // the temporary
2334   let tmp = foo(); // the temporary                                                                                                                     2329   bar(&tmp)
2335   bar(&tmp)                                                                                                                                             2330 }; // <-- tmp is freed as we exit this block
2336 }; // <-- tmp is freed as we exit this block                                                                                                            2331 let q = p;
2337 let q = p;                                                                                                                                              2332 ```
2338 ```                                                                                                                                                     2333 
2339                                                                                                                                                         2334 Whenever a temporary is created, it is automatically dropped (freed)
2340 Whenever a temporary is created, it is automatically dropped (freed)                                                                                    2335 according to fixed rules. Ordinarily, the temporary is dropped
2341 according to fixed rules. Ordinarily, the temporary is dropped                                                                                          2336 at the end of the enclosing statement -- in this case, after the `let`.
2342 at the end of the enclosing statement -- in this case, after the `let`.                                                                                 2337 This is illustrated in the example above by showing that `tmp` would
2343 This is illustrated in the example above by showing that `tmp` would                                                                                    2338 be freed as we exit the block.
2344 be freed as we exit the block.                                                                                                                          2339 
2345                                                                                                                                                         2340 To fix this problem, you need to create a local variable
2346 To fix this problem, you need to create a local variable                                                                                                2341 to store the value in rather than relying on a temporary.
2347 to store the value in rather than relying on a temporary.                                                                                               2342 For example, you might change the original program to
2348 For example, you might change the original program to                                                                                                   2343 the following:
2349 the following:                                                                                                                                          2344 
2350                                                                                                                                                         2345 ```
2351 ```                                                                                                                                                     2346 fn foo() -> i32 { 22 }
2352 fn foo() -> i32 { 22 }                                                                                                                                  2347 fn bar(x: &i32) -> &i32 { x }
2353 fn bar(x: &i32) -> &i32 { x }                                                                                                                           2348 let value = foo(); // dropped at the end of the enclosing block
2354 let value = foo(); // dropped at the end of the enclosing block                                                                                         2349 let p = bar(&value);
2355 let p = bar(&value);                                                                                                                                    2350 let q = *p;
2356 let q = *p;                                                                                                                                             2351 ```
2357 ```                                                                                                                                                     2352 
2358                                                                                                                                                         2353 By introducing the explicit `let value`, we allocate storage
2359 By introducing the explicit `let value`, we allocate storage                                                                                            2354 that will last until the end of the enclosing block (when `value`
2360 that will last until the end of the enclosing block (when `value`                                                                                       2355 goes out of scope). When we borrow `&value`, we are borrowing a
2361 goes out of scope). When we borrow `&value`, we are borrowing a                                                                                         2356 local variable that already exists, and hence no temporary is created.
2362 local variable that already exists, and hence no temporary is created.                                                                                  2357 
2363                                                                                                                                                         2358 Temporaries are not always dropped at the end of the enclosing
2364 Temporaries are not always dropped at the end of the enclosing                                                                                          2359 statement. In simple cases where the `&` expression is immediately
2365 statement. In simple cases where the `&` expression is immediately                                                                                      2360 stored into a variable, the compiler will automatically extend
2366 stored into a variable, the compiler will automatically extend                                                                                          2361 the lifetime of the temporary until the end of the enclosing
2367 the lifetime of the temporary until the end of the enclosing                                                                                            2362 block. Therefore, an alternative way to fix the original
2368 block. Therefore, an alternative way to fix the original                                                                                                2363 program is to write `let tmp = &foo()` and not `let tmp = foo()`:
2369 program is to write `let tmp = &foo()` and not `let tmp = foo()`:                                                                                       2364 
2370                                                                                                                                                         2365 ```
2371 ```                                                                                                                                                     2366 fn foo() -> i32 { 22 }
2372 fn foo() -> i32 { 22 }                                                                                                                                  2367 fn bar(x: &i32) -> &i32 { x }
2373 fn bar(x: &i32) -> &i32 { x }                                                                                                                           2368 let value = &foo();
2374 let value = &foo();                                                                                                                                     2369 let p = bar(value);
2375 let p = bar(value);                                                                                                                                     2370 let q = *p;
2376 let q = *p;                                                                                                                                             2371 ```
2377 ```                                                                                                                                                     2372 
2378                                                                                                                                                         2373 Here, we are still borrowing `foo()`, but as the borrow is assigned
2379 Here, we are still borrowing `foo()`, but as the borrow is assigned                                                                                     2374 directly into a variable, the temporary will not be dropped until
2380 directly into a variable, the temporary will not be dropped until                                                                                       2375 the end of the enclosing block. Similar rules apply when temporaries
2381 the end of the enclosing block. Similar rules apply when temporaries                                                                                    2376 are stored into aggregate structures like a tuple or struct:
2382 are stored into aggregate structures like a tuple or struct:                                                                                            2377 
2383                                                                                                                                                         2378 ```
2384 ```                                                                                                                                                     2379 // Here, two temporaries are created, but
2385 // Here, two temporaries are created, but                                                                                                               2380 // as they are stored directly into `value`,
2386 // as they are stored directly into `value`,                                                                                                            2381 // they are not dropped until the end of the
2387 // they are not dropped until the end of the                                                                                                            2382 // enclosing block.
2388 // enclosing block.                                                                                                                                     2383 fn foo() -> i32 { 22 }
2389 fn foo() -> i32 { 22 }                                                                                                                                  2384 let value = (&foo(), &foo());
2390 let value = (&foo(), &foo());                                                                                                                           2385 ```
2391 ```                                                                                                                                                     .... 
2392 "##,                                                                                                                                                    2386 "##,

