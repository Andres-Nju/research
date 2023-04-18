File_Code/rust/3f9d834fb3/error_codes/error_codes_after.rs --- Rust
  6 E0038: r##"                                                                                                                                                6 E0038: r##"
  7 Trait objects like `Box<Trait>` can only be constructed when certain                                                                                       7 Trait objects like `Box<Trait>` can only be constructed when certain
  8 requirements are satisfied by the trait in question.                                                                                                       8 requirements are satisfied by the trait in question.
  9                                                                                                                                                            9 
 10 Trait objects are a form of dynamic dispatch and use a dynamically sized type                                                                             10 Trait objects are a form of dynamic dispatch and use a dynamically sized type
 11 for the inner type. So, for a given trait `Trait`, when `Trait` is treated as a                                                                           11 for the inner type. So, for a given trait `Trait`, when `Trait` is treated as a
 12 type, as in `Box<Trait>`, the inner type is 'unsized'. In such cases the boxed                                                                            12 type, as in `Box<Trait>`, the inner type is 'unsized'. In such cases the boxed
 13 pointer is a 'fat pointer' that contains an extra pointer to a table of methods                                                                           13 pointer is a 'fat pointer' that contains an extra pointer to a table of methods
 14 (among other things) for dynamic dispatch. This design mandates some                                                                                      14 (among other things) for dynamic dispatch. This design mandates some
 15 restrictions on the types of traits that are allowed to be used in trait                                                                                  15 restrictions on the types of traits that are allowed to be used in trait
 16 objects, which are collectively termed as 'object safety' rules.                                                                                          16 objects, which are collectively termed as 'object safety' rules.
 17                                                                                                                                                           17 
 18 Attempting to create a trait object for a non object-safe trait will trigger                                                                              18 Attempting to create a trait object for a non object-safe trait will trigger
 19 this error.                                                                                                                                               19 this error.
 20                                                                                                                                                           20 
 21 There are various rules:                                                                                                                                  21 There are various rules:
 22                                                                                                                                                           22 
 23 ### The trait cannot require `Self: Sized`                                                                                                                23 ### The trait cannot require `Self: Sized`
 24                                                                                                                                                           24 
 25 When `Trait` is treated as a type, the type does not implement the special                                                                                25 When `Trait` is treated as a type, the type does not implement the special
 26 `Sized` trait, because the type does not have a known size at compile time and                                                                            26 `Sized` trait, because the type does not have a known size at compile time and
 27 can only be accessed behind a pointer. Thus, if we have a trait like the                                                                                  27 can only be accessed behind a pointer. Thus, if we have a trait like the
 28 following:                                                                                                                                                28 following:
 29                                                                                                                                                           29 
 30 ```                                                                                                                                                       30 ```
 31 trait Foo where Self: Sized {                                                                                                                             31 trait Foo where Self: Sized {
 32                                                                                                                                                           32 
 33 }                                                                                                                                                         33 }
 34 ```                                                                                                                                                       34 ```
 35                                                                                                                                                           35 
 36 We cannot create an object of type `Box<Foo>` or `&Foo` since in this case                                                                                36 We cannot create an object of type `Box<Foo>` or `&Foo` since in this case
 37 `Self` would not be `Sized`.                                                                                                                              37 `Self` would not be `Sized`.
 38                                                                                                                                                           38 
 39 Generally, `Self: Sized` is used to indicate that the trait should not be used                                                                            39 Generally, `Self: Sized` is used to indicate that the trait should not be used
 40 as a trait object. If the trait comes from your own crate, consider removing                                                                              40 as a trait object. If the trait comes from your own crate, consider removing
 41 this restriction.                                                                                                                                         41 this restriction.
 42                                                                                                                                                           42 
 43 ### Method references the `Self` type in its parameters or return type                                                                                    43 ### Method references the `Self` type in its parameters or return type
 44                                                                                                                                                           44 
 45 This happens when a trait has a method like the following:                                                                                                45 This happens when a trait has a method like the following:
 46                                                                                                                                                           46 
 47 ```                                                                                                                                                       47 ```
 48 trait Trait {                                                                                                                                             48 trait Trait {
 49     fn foo(&self) -> Self;                                                                                                                                49     fn foo(&self) -> Self;
 50 }                                                                                                                                                         50 }
 51                                                                                                                                                           51 
 52 impl Trait for String {                                                                                                                                   52 impl Trait for String {
 53     fn foo(&self) -> Self {                                                                                                                               53     fn foo(&self) -> Self {
 54         "hi".to_owned()                                                                                                                                   54         "hi".to_owned()
 55     }                                                                                                                                                     55     }
 56 }                                                                                                                                                         56 }
 57                                                                                                                                                           57 
 58 impl Trait for u8 {                                                                                                                                       58 impl Trait for u8 {
 59     fn foo(&self) -> Self {                                                                                                                               59     fn foo(&self) -> Self {
 60         1                                                                                                                                                 60         1
 61     }                                                                                                                                                     61     }
 62 }                                                                                                                                                         62 }
 63 ```                                                                                                                                                       63 ```
 64                                                                                                                                                           64 
 65 (Note that `&self` and `&mut self` are okay, it's additional `Self` types which                                                                           65 (Note that `&self` and `&mut self` are okay, it's additional `Self` types which
 66 cause this problem.)                                                                                                                                      66 cause this problem.)
 67                                                                                                                                                           67 
 68 In such a case, the compiler cannot predict the return type of `foo()` in a                                                                               68 In such a case, the compiler cannot predict the return type of `foo()` in a
 69 situation like the following:                                                                                                                             69 situation like the following:
 70                                                                                                                                                           70 
 71 ```compile_fail                                                                                                                                           71 ```compile_fail
 72 trait Trait {                                                                                                                                             72 trait Trait {
 73     fn foo(&self) -> Self;                                                                                                                                73     fn foo(&self) -> Self;
 74 }                                                                                                                                                         74 }
 75                                                                                                                                                           75 
 76 fn call_foo(x: Box<Trait>) {                                                                                                                              76 fn call_foo(x: Box<Trait>) {
 77     let y = x.foo(); // What type is y?                                                                                                                   77     let y = x.foo(); // What type is y?
 78     // ...                                                                                                                                                78     // ...
 79 }                                                                                                                                                         79 }
 80 ```                                                                                                                                                       80 ```
 81                                                                                                                                                           81 
 82 If only some methods aren't object-safe, you can add a `where Self: Sized` bound                                                                          82 If only some methods aren't object-safe, you can add a `where Self: Sized` bound
 83 on them to mark them as explicitly unavailable to trait objects. The                                                                                      83 on them to mark them as explicitly unavailable to trait objects. The
 84 functionality will still be available to all other implementers, including                                                                                84 functionality will still be available to all other implementers, including
 85 `Box<Trait>` which is itself sized (assuming you `impl Trait for Box<Trait>`).                                                                            85 `Box<Trait>` which is itself sized (assuming you `impl Trait for Box<Trait>`).
 86                                                                                                                                                           86 
 87 ```                                                                                                                                                       87 ```
 88 trait Trait {                                                                                                                                             88 trait Trait {
 89     fn foo(&self) -> Self where Self: Sized;                                                                                                              89     fn foo(&self) -> Self where Self: Sized;
 90     // more functions                                                                                                                                     90     // more functions
 91 }                                                                                                                                                         91 }
 92 ```                                                                                                                                                       92 ```
 93                                                                                                                                                           93 
 94 Now, `foo()` can no longer be called on a trait object, but you will now be                                                                               94 Now, `foo()` can no longer be called on a trait object, but you will now be
 95 allowed to make a trait object, and that will be able to call any object-safe                                                                             95 allowed to make a trait object, and that will be able to call any object-safe
 96 methods. With such a bound, one can still call `foo()` on types implementing                                                                              96 methods. With such a bound, one can still call `foo()` on types implementing
 97 that trait that aren't behind trait objects.                                                                                                              97 that trait that aren't behind trait objects.
 98                                                                                                                                                           98 
 99 ### Method has generic type parameters                                                                                                                    99 ### Method has generic type parameters
100                                                                                                                                                          100 
101 As mentioned before, trait objects contain pointers to method tables. So, if we                                                                          101 As mentioned before, trait objects contain pointers to method tables. So, if we
102 have:                                                                                                                                                    102 have:
103                                                                                                                                                          103 
104 ```                                                                                                                                                      104 ```
105 trait Trait {                                                                                                                                            105 trait Trait {
106     fn foo(&self);                                                                                                                                       106     fn foo(&self);
107 }                                                                                                                                                        107 }
108                                                                                                                                                          108 
109 impl Trait for String {                                                                                                                                  109 impl Trait for String {
110     fn foo(&self) {                                                                                                                                      110     fn foo(&self) {
111         // implementation 1                                                                                                                              111         // implementation 1
112     }                                                                                                                                                    112     }
113 }                                                                                                                                                        113 }
114                                                                                                                                                          114 
115 impl Trait for u8 {                                                                                                                                      115 impl Trait for u8 {
116     fn foo(&self) {                                                                                                                                      116     fn foo(&self) {
117         // implementation 2                                                                                                                              117         // implementation 2
118     }                                                                                                                                                    118     }
119 }                                                                                                                                                        119 }
120 // ...                                                                                                                                                   120 // ...
121 ```                                                                                                                                                      121 ```
122                                                                                                                                                          122 
123 At compile time each implementation of `Trait` will produce a table containing                                                                           123 At compile time each implementation of `Trait` will produce a table containing
124 the various methods (and other items) related to the implementation.                                                                                     124 the various methods (and other items) related to the implementation.
125                                                                                                                                                          125 
126 This works fine, but when the method gains generic parameters, we can have a                                                                             126 This works fine, but when the method gains generic parameters, we can have a
127 problem.                                                                                                                                                 127 problem.
128                                                                                                                                                          128 
129 Usually, generic parameters get _monomorphized_. For example, if I have                                                                                  129 Usually, generic parameters get _monomorphized_. For example, if I have
130                                                                                                                                                          130 
131 ```                                                                                                                                                      131 ```
132 fn foo<T>(x: T) {                                                                                                                                        132 fn foo<T>(x: T) {
133     // ...                                                                                                                                               133     // ...
134 }                                                                                                                                                        134 }
135 ```                                                                                                                                                      135 ```
136                                                                                                                                                          136 
137 The machine code for `foo::<u8>()`, `foo::<bool>()`, `foo::<String>()`, or any                                                                           137 The machine code for `foo::<u8>()`, `foo::<bool>()`, `foo::<String>()`, or any
138 other type substitution is different. Hence the compiler generates the                                                                                   138 other type substitution is different. Hence the compiler generates the
139 implementation on-demand. If you call `foo()` with a `bool` parameter, the                                                                               139 implementation on-demand. If you call `foo()` with a `bool` parameter, the
140 compiler will only generate code for `foo::<bool>()`. When we have additional                                                                            140 compiler will only generate code for `foo::<bool>()`. When we have additional
141 type parameters, the number of monomorphized implementations the compiler                                                                                141 type parameters, the number of monomorphized implementations the compiler
142 generates does not grow drastically, since the compiler will only generate an                                                                            142 generates does not grow drastically, since the compiler will only generate an
143 implementation if the function is called with unparametrized substitutions                                                                               143 implementation if the function is called with unparametrized substitutions
144 (i.e., substitutions where none of the substituted types are themselves                                                                                  144 (i.e., substitutions where none of the substituted types are themselves
145 parametrized).                                                                                                                                           145 parametrized).
146                                                                                                                                                          146 
147 However, with trait objects we have to make a table containing _every_ object                                                                            147 However, with trait objects we have to make a table containing _every_ object
148 that implements the trait. Now, if it has type parameters, we need to add                                                                                148 that implements the trait. Now, if it has type parameters, we need to add
149 implementations for every type that implements the trait, and there could                                                                                149 implementations for every type that implements the trait, and there could
150 theoretically be an infinite number of types.                                                                                                            150 theoretically be an infinite number of types.
151                                                                                                                                                          151 
152 For example, with:                                                                                                                                       152 For example, with:
153                                                                                                                                                          153 
154 ```                                                                                                                                                      154 ```
155 trait Trait {                                                                                                                                            155 trait Trait {
156     fn foo<T>(&self, on: T);                                                                                                                             156     fn foo<T>(&self, on: T);
157     // more methods                                                                                                                                      157     // more methods
158 }                                                                                                                                                        158 }
159                                                                                                                                                          159 
160 impl Trait for String {                                                                                                                                  160 impl Trait for String {
161     fn foo<T>(&self, on: T) {                                                                                                                            161     fn foo<T>(&self, on: T) {
162         // implementation 1                                                                                                                              162         // implementation 1
163     }                                                                                                                                                    163     }
164 }                                                                                                                                                        164 }
165                                                                                                                                                          165 
166 impl Trait for u8 {                                                                                                                                      166 impl Trait for u8 {
167     fn foo<T>(&self, on: T) {                                                                                                                            167     fn foo<T>(&self, on: T) {
168         // implementation 2                                                                                                                              168         // implementation 2
169     }                                                                                                                                                    169     }
170 }                                                                                                                                                        170 }
171                                                                                                                                                          171 
172 // 8 more implementations                                                                                                                                172 // 8 more implementations
173 ```                                                                                                                                                      173 ```
174                                                                                                                                                          174 
175 Now, if we have the following code:                                                                                                                      175 Now, if we have the following code:
176                                                                                                                                                          176 
177 ```compile_fail,E0038                                                                                                                                    177 ```compile_fail,E0038
178 # trait Trait { fn foo<T>(&self, on: T); }                                                                                                               178 # trait Trait { fn foo<T>(&self, on: T); }
179 # impl Trait for String { fn foo<T>(&self, on: T) {} }                                                                                                   179 # impl Trait for String { fn foo<T>(&self, on: T) {} }
180 # impl Trait for u8 { fn foo<T>(&self, on: T) {} }                                                                                                       180 # impl Trait for u8 { fn foo<T>(&self, on: T) {} }
181 # impl Trait for bool { fn foo<T>(&self, on: T) {} }                                                                                                     181 # impl Trait for bool { fn foo<T>(&self, on: T) {} }
182 # // etc.                                                                                                                                                182 # // etc.
183 fn call_foo(thing: Box<Trait>) {                                                                                                                         183 fn call_foo(thing: Box<Trait>) {
184     thing.foo(true); // this could be any one of the 8 types above                                                                                       184     thing.foo(true); // this could be any one of the 8 types above
185     thing.foo(1);                                                                                                                                        185     thing.foo(1);
186     thing.foo("hello");                                                                                                                                  186     thing.foo("hello");
187 }                                                                                                                                                        187 }
188 ```                                                                                                                                                      188 ```
189                                                                                                                                                          189 
190 We don't just need to create a table of all implementations of all methods of                                                                            190 We don't just need to create a table of all implementations of all methods of
191 `Trait`, we need to create such a table, for each different type fed to                                                                                  191 `Trait`, we need to create such a table, for each different type fed to
192 `foo()`. In this case this turns out to be (10 types implementing `Trait`)*(3                                                                            192 `foo()`. In this case this turns out to be (10 types implementing `Trait`)*(3
193 types being fed to `foo()`) = 30 implementations!                                                                                                        193 types being fed to `foo()`) = 30 implementations!
194                                                                                                                                                          194 
195 With real world traits these numbers can grow drastically.                                                                                               195 With real world traits these numbers can grow drastically.
196                                                                                                                                                          196 
197 To fix this, it is suggested to use a `where Self: Sized` bound similar to the                                                                           197 To fix this, it is suggested to use a `where Self: Sized` bound similar to the
198 fix for the sub-error above if you do not intend to call the method with type                                                                            198 fix for the sub-error above if you do not intend to call the method with type
199 parameters:                                                                                                                                              199 parameters:
200                                                                                                                                                          200 
201 ```                                                                                                                                                      201 ```
202 trait Trait {                                                                                                                                            202 trait Trait {
203     fn foo<T>(&self, on: T) where Self: Sized;                                                                                                           203     fn foo<T>(&self, on: T) where Self: Sized;
204     // more methods                                                                                                                                      204     // more methods
205 }                                                                                                                                                        205 }
206 ```                                                                                                                                                      206 ```
207                                                                                                                                                          207 
208 If this is not an option, consider replacing the type parameter with another                                                                             208 If this is not an option, consider replacing the type parameter with another
209 trait object (e.g., if `T: OtherTrait`, use `on: Box<OtherTrait>`). If the                                                                               209 trait object (e.g., if `T: OtherTrait`, use `on: Box<OtherTrait>`). If the
210 number of types you intend to feed to this method is limited, consider manually                                                                          210 number of types you intend to feed to this method is limited, consider manually
211 listing out the methods of different types.                                                                                                              211 listing out the methods of different types.
212                                                                                                                                                          212 
213 ### Method has no receiver                                                                                                                               213 ### Method has no receiver
214                                                                                                                                                          214 
215 Methods that do not take a `self` parameter can't be called since there won't be                                                                         215 Methods that do not take a `self` parameter can't be called since there won't be
216 a way to get a pointer to the method table for them.                                                                                                     216 a way to get a pointer to the method table for them.
217                                                                                                                                                          217 
218 ```                                                                                                                                                      218 ```
219 trait Foo {                                                                                                                                              219 trait Foo {
220     fn foo() -> u8;                                                                                                                                      220     fn foo() -> u8;
221 }                                                                                                                                                        221 }
222 ```                                                                                                                                                      222 ```
223                                                                                                                                                          223 
224 This could be called as `<Foo as Foo>::foo()`, which would not be able to pick                                                                           224 This could be called as `<Foo as Foo>::foo()`, which would not be able to pick
225 an implementation.                                                                                                                                       225 an implementation.
226                                                                                                                                                          226 
227 Adding a `Self: Sized` bound to these methods will generally make this compile.                                                                          227 Adding a `Self: Sized` bound to these methods will generally make this compile.
228                                                                                                                                                          228 
229 ```                                                                                                                                                      229 ```
230 trait Foo {                                                                                                                                              230 trait Foo {
231     fn foo() -> u8 where Self: Sized;                                                                                                                    231     fn foo() -> u8 where Self: Sized;
232 }                                                                                                                                                        232 }
233 ```                                                                                                                                                      233 ```
234                                                                                                                                                          234 
235 ### The trait cannot contain associated constants                                                                                                        235 ### The trait cannot contain associated constants
236                                                                                                                                                          236 
237 Just like static functions, associated constants aren't stored on the method                                                                             237 Just like static functions, associated constants aren't stored on the method
238 table. If the trait or any subtrait contain an associated constant, they cannot                                                                          238 table. If the trait or any subtrait contain an associated constant, they cannot
239 be made into an object.                                                                                                                                  239 be made into an object.
240                                                                                                                                                          240 
241 ```compile_fail,E0038                                                                                                                                    241 ```compile_fail,E0038
242 trait Foo {                                                                                                                                              242 trait Foo {
243     const X: i32;                                                                                                                                        243     const X: i32;
244 }                                                                                                                                                        244 }
245                                                                                                                                                          245 
246 impl Foo {}                                                                                                                                              246 impl Foo {}
247 ```                                                                                                                                                      247 ```
248                                                                                                                                                          248 
249 A simple workaround is to use a helper method instead:                                                                                                   249 A simple workaround is to use a helper method instead:
250                                                                                                                                                          250 
251 ```                                                                                                                                                      251 ```
252 trait Foo {                                                                                                                                              252 trait Foo {
253     fn x(&self) -> i32;                                                                                                                                  253     fn x(&self) -> i32;
254 }                                                                                                                                                        254 }
255 ```                                                                                                                                                      255 ```
256                                                                                                                                                          256 
257 ### The trait cannot use `Self` as a type parameter in the supertrait listing                                                                            257 ### The trait cannot use `Self` as a type parameter in the supertrait listing
258                                                                                                                                                          258 
259 This is similar to the second sub-error, but subtler. It happens in situations                                                                           259 This is similar to the second sub-error, but subtler. It happens in situations
260 like the following:                                                                                                                                      260 like the following:
261                                                                                                                                                          261 
262 ```                                                                                                                                                      262 ```compile_fail,E0038
263 trait Super<A: ?Sized> {}                                                                                                                                263 trait Super<A: ?Sized> {}
264                                                                                                                                                          264 
265 trait Trait: Super<Self> {                                                                                                                               265 trait Trait: Super<Self> {
266 }                                                                                                                                                        266 }
267                                                                                                                                                          267 
268 struct Foo;                                                                                                                                              268 struct Foo;
269                                                                                                                                                          269 
270 impl Super<Foo> for Foo{}                                                                                                                                270 impl Super<Foo> for Foo{}
271                                                                                                                                                          271 
272 impl Trait for Foo {}                                                                                                                                    272 impl Trait for Foo {}
273 ```                                                                                                                                                      273 
274                                                                                                                                                          274 fn main() {
275 Here, the supertrait might have methods as follows:                                                                                                      275     let x: Box<dyn Trait>;
276                                                                                                                                                          276 }
277 ```                                                                                                                                                      277 ```
278 trait Super<A: ?Sized> {                                                                                                                                 278 
279     fn get_a(&self) -> &A; // note that this is object safe!                                                                                             279 Here, the supertrait might have methods as follows:
280 }                                                                                                                                                        280 
281 ```                                                                                                                                                      281 ```
282                                                                                                                                                          282 trait Super<A: ?Sized> {
283 If the trait `Trait` was deriving from something like `Super<String>` or                                                                                 283     fn get_a(&self) -> &A; // note that this is object safe!
284 `Super<T>` (where `Foo` itself is `Foo<T>`), this is okay, because given a type                                                                          284 }
285 `get_a()` will definitely return an object of that type.                                                                                                 285 ```
286                                                                                                                                                          286 
287 However, if it derives from `Super<Self>`, even though `Super` is object safe,                                                                           287 If the trait `Trait` was deriving from something like `Super<String>` or
288 the method `get_a()` would return an object of unknown type when called on the                                                                           288 `Super<T>` (where `Foo` itself is `Foo<T>`), this is okay, because given a type
289 function. `Self` type parameters let us make object safe traits no longer safe,                                                                          289 `get_a()` will definitely return an object of that type.
290 so they are forbidden when specifying supertraits.                                                                                                       290 
291                                                                                                                                                          291 However, if it derives from `Super<Self>`, even though `Super` is object safe,
292 There's no easy fix for this, generally code will need to be refactored so that                                                                          292 the method `get_a()` would return an object of unknown type when called on the
293 you no longer need to derive from `Super<Self>`.                                                                                                         293 function. `Self` type parameters let us make object safe traits no longer safe,
...                                                                                                                                                          294 so they are forbidden when specifying supertraits.
...                                                                                                                                                          295 
...                                                                                                                                                          296 There's no easy fix for this, generally code will need to be refactored so that
...                                                                                                                                                          297 you no longer need to derive from `Super<Self>`.
294 "##,                                                                                                                                                     298 "##,

