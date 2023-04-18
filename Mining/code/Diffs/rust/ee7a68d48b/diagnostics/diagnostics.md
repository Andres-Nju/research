File_Code/rust/ee7a68d48b/diagnostics/diagnostics_after.rs --- Rust
 28 E0038: r##"                                                                                                                                               28 E0038: r##"
 29 Trait objects like `Box<Trait>` can only be constructed when certain                                                                                      29 Trait objects like `Box<Trait>` can only be constructed when certain
 30 requirements are satisfied by the trait in question.                                                                                                      30 requirements are satisfied by the trait in question.
 31                                                                                                                                                           31 
 32 Trait objects are a form of dynamic dispatch and use a dynamically sized type                                                                             32 Trait objects are a form of dynamic dispatch and use a dynamically sized type
 33 for the inner type. So, for a given trait `Trait`, when `Trait` is treated as a                                                                           33 for the inner type. So, for a given trait `Trait`, when `Trait` is treated as a
 34 type, as in `Box<Trait>`, the inner type is 'unsized'. In such cases the boxed                                                                            34 type, as in `Box<Trait>`, the inner type is 'unsized'. In such cases the boxed
 35 pointer is a 'fat pointer' that contains an extra pointer to a table of methods                                                                           35 pointer is a 'fat pointer' that contains an extra pointer to a table of methods
 36 (among other things) for dynamic dispatch. This design mandates some                                                                                      36 (among other things) for dynamic dispatch. This design mandates some
 37 restrictions on the types of traits that are allowed to be used in trait                                                                                  37 restrictions on the types of traits that are allowed to be used in trait
 38 objects, which are collectively termed as 'object safety' rules.                                                                                          38 objects, which are collectively termed as 'object safety' rules.
 39                                                                                                                                                           39 
 40 Attempting to create a trait object for a non object-safe trait will trigger                                                                              40 Attempting to create a trait object for a non object-safe trait will trigger
 41 this error.                                                                                                                                               41 this error.
 42                                                                                                                                                           42 
 43 There are various rules:                                                                                                                                  43 There are various rules:
 44                                                                                                                                                           44 
 45 ### The trait cannot require `Self: Sized`                                                                                                                45 ### The trait cannot require `Self: Sized`
 46                                                                                                                                                           46 
 47 When `Trait` is treated as a type, the type does not implement the special                                                                                47 When `Trait` is treated as a type, the type does not implement the special
 48 `Sized` trait, because the type does not have a known size at compile time and                                                                            48 `Sized` trait, because the type does not have a known size at compile time and
 49 can only be accessed behind a pointer. Thus, if we have a trait like the                                                                                  49 can only be accessed behind a pointer. Thus, if we have a trait like the
 50 following:                                                                                                                                                50 following:
 51                                                                                                                                                           51 
 52 ```                                                                                                                                                       52 ```
 53 trait Foo where Self: Sized {                                                                                                                             53 trait Foo where Self: Sized {
 54                                                                                                                                                           54 
 55 }                                                                                                                                                         55 }
 56 ```                                                                                                                                                       56 ```
 57                                                                                                                                                           57 
 58 We cannot create an object of type `Box<Foo>` or `&Foo` since in this case                                                                                58 We cannot create an object of type `Box<Foo>` or `&Foo` since in this case
 59 `Self` would not be `Sized`.                                                                                                                              59 `Self` would not be `Sized`.
 60                                                                                                                                                           60 
 61 Generally, `Self : Sized` is used to indicate that the trait should not be used                                                                           61 Generally, `Self : Sized` is used to indicate that the trait should not be used
 62 as a trait object. If the trait comes from your own crate, consider removing                                                                              62 as a trait object. If the trait comes from your own crate, consider removing
 63 this restriction.                                                                                                                                         63 this restriction.
 64                                                                                                                                                           64 
 65 ### Method references the `Self` type in its arguments or return type                                                                                     65 ### Method references the `Self` type in its arguments or return type
 66                                                                                                                                                           66 
 67 This happens when a trait has a method like the following:                                                                                                67 This happens when a trait has a method like the following:
 68                                                                                                                                                           68 
 69 ```compile_fail                                                                                                                                           69 ```compile_fail
 70 trait Trait {                                                                                                                                             70 trait Trait {
 71     fn foo(&self) -> Self;                                                                                                                                71     fn foo(&self) -> Self;
 72 }                                                                                                                                                         72 }
 73                                                                                                                                                           73 
 74 impl Trait for String {                                                                                                                                   74 impl Trait for String {
 75     fn foo(&self) -> Self {                                                                                                                               75     fn foo(&self) -> Self {
 76         "hi".to_owned()                                                                                                                                   76         "hi".to_owned()
 77     }                                                                                                                                                     77     }
 78 }                                                                                                                                                         78 }
 79                                                                                                                                                           79 
 80 impl Trait for u8 {                                                                                                                                       80 impl Trait for u8 {
 81     fn foo(&self) -> Self {                                                                                                                               81     fn foo(&self) -> Self {
 82         1                                                                                                                                                 82         1
 83     }                                                                                                                                                     83     }
 84 }                                                                                                                                                         84 }
 85 ```                                                                                                                                                       85 ```
 86                                                                                                                                                           86 
 87 (Note that `&self` and `&mut self` are okay, it's additional `Self` types which                                                                           87 (Note that `&self` and `&mut self` are okay, it's additional `Self` types which
 88 cause this problem.)                                                                                                                                      88 cause this problem.)
 89                                                                                                                                                           89 
 90 In such a case, the compiler cannot predict the return type of `foo()` in a                                                                               90 In such a case, the compiler cannot predict the return type of `foo()` in a
 91 situation like the following:                                                                                                                             91 situation like the following:
 92                                                                                                                                                           92 
 93 ```compile_fail                                                                                                                                           93 ```compile_fail
 94 trait Trait {                                                                                                                                             94 trait Trait {
 95     fn foo(&self) -> Self;                                                                                                                                95     fn foo(&self) -> Self;
 96 }                                                                                                                                                         96 }
 97                                                                                                                                                           97 
 98 fn call_foo(x: Box<Trait>) {                                                                                                                              98 fn call_foo(x: Box<Trait>) {
 99     let y = x.foo(); // What type is y?                                                                                                                   99     let y = x.foo(); // What type is y?
100     // ...                                                                                                                                               100     // ...
101 }                                                                                                                                                        101 }
102 ```                                                                                                                                                      102 ```
103                                                                                                                                                          103 
104 If only some methods aren't object-safe, you can add a `where Self: Sized` bound                                                                         104 If only some methods aren't object-safe, you can add a `where Self: Sized` bound
105 on them to mark them as explicitly unavailable to trait objects. The                                                                                     105 on them to mark them as explicitly unavailable to trait objects. The
106 functionality will still be available to all other implementers, including                                                                               106 functionality will still be available to all other implementers, including
107 `Box<Trait>` which is itself sized (assuming you `impl Trait for Box<Trait>`).                                                                           107 `Box<Trait>` which is itself sized (assuming you `impl Trait for Box<Trait>`).
108                                                                                                                                                          108 
109 ```                                                                                                                                                      109 ```
110 trait Trait {                                                                                                                                            110 trait Trait {
111     fn foo(&self) -> Self where Self: Sized;                                                                                                             111     fn foo(&self) -> Self where Self: Sized;
112     // more functions                                                                                                                                    112     // more functions
113 }                                                                                                                                                        113 }
114 ```                                                                                                                                                      114 ```
115                                                                                                                                                          115 
116 Now, `foo()` can no longer be called on a trait object, but you will now be                                                                              116 Now, `foo()` can no longer be called on a trait object, but you will now be
117 allowed to make a trait object, and that will be able to call any object-safe                                                                            117 allowed to make a trait object, and that will be able to call any object-safe
118 methods". With such a bound, one can still call `foo()` on types implementing                                                                            118 methods. With such a bound, one can still call `foo()` on types implementing
119 that trait that aren't behind trait objects.                                                                                                             119 that trait that aren't behind trait objects.
120                                                                                                                                                          120 
121 ### Method has generic type parameters                                                                                                                   121 ### Method has generic type parameters
122                                                                                                                                                          122 
123 As mentioned before, trait objects contain pointers to method tables. So, if we                                                                          123 As mentioned before, trait objects contain pointers to method tables. So, if we
124 have:                                                                                                                                                    124 have:
125                                                                                                                                                          125 
126 ```                                                                                                                                                      126 ```
127 trait Trait {                                                                                                                                            127 trait Trait {
128     fn foo(&self);                                                                                                                                       128     fn foo(&self);
129 }                                                                                                                                                        129 }
130                                                                                                                                                          130 
131 impl Trait for String {                                                                                                                                  131 impl Trait for String {
132     fn foo(&self) {                                                                                                                                      132     fn foo(&self) {
133         // implementation 1                                                                                                                              133         // implementation 1
134     }                                                                                                                                                    134     }
135 }                                                                                                                                                        135 }
136                                                                                                                                                          136 
137 impl Trait for u8 {                                                                                                                                      137 impl Trait for u8 {
138     fn foo(&self) {                                                                                                                                      138     fn foo(&self) {
139         // implementation 2                                                                                                                              139         // implementation 2
140     }                                                                                                                                                    140     }
141 }                                                                                                                                                        141 }
142 // ...                                                                                                                                                   142 // ...
143 ```                                                                                                                                                      143 ```
144                                                                                                                                                          144 
145 At compile time each implementation of `Trait` will produce a table containing                                                                           145 At compile time each implementation of `Trait` will produce a table containing
146 the various methods (and other items) related to the implementation.                                                                                     146 the various methods (and other items) related to the implementation.
147                                                                                                                                                          147 
148 This works fine, but when the method gains generic parameters, we can have a                                                                             148 This works fine, but when the method gains generic parameters, we can have a
149 problem.                                                                                                                                                 149 problem.
150                                                                                                                                                          150 
151 Usually, generic parameters get _monomorphized_. For example, if I have                                                                                  151 Usually, generic parameters get _monomorphized_. For example, if I have
152                                                                                                                                                          152 
153 ```                                                                                                                                                      153 ```
154 fn foo<T>(x: T) {                                                                                                                                        154 fn foo<T>(x: T) {
155     // ...                                                                                                                                               155     // ...
156 }                                                                                                                                                        156 }
157 ```                                                                                                                                                      157 ```
158                                                                                                                                                          158 
159 The machine code for `foo::<u8>()`, `foo::<bool>()`, `foo::<String>()`, or any                                                                           159 The machine code for `foo::<u8>()`, `foo::<bool>()`, `foo::<String>()`, or any
160 other type substitution is different. Hence the compiler generates the                                                                                   160 other type substitution is different. Hence the compiler generates the
161 implementation on-demand. If you call `foo()` with a `bool` parameter, the                                                                               161 implementation on-demand. If you call `foo()` with a `bool` parameter, the
162 compiler will only generate code for `foo::<bool>()`. When we have additional                                                                            162 compiler will only generate code for `foo::<bool>()`. When we have additional
163 type parameters, the number of monomorphized implementations the compiler                                                                                163 type parameters, the number of monomorphized implementations the compiler
164 generates does not grow drastically, since the compiler will only generate an                                                                            164 generates does not grow drastically, since the compiler will only generate an
165 implementation if the function is called with unparametrized substitutions                                                                               165 implementation if the function is called with unparametrized substitutions
166 (i.e., substitutions where none of the substituted types are themselves                                                                                  166 (i.e., substitutions where none of the substituted types are themselves
167 parametrized).                                                                                                                                           167 parametrized).
168                                                                                                                                                          168 
169 However, with trait objects we have to make a table containing _every_ object                                                                            169 However, with trait objects we have to make a table containing _every_ object
170 that implements the trait. Now, if it has type parameters, we need to add                                                                                170 that implements the trait. Now, if it has type parameters, we need to add
171 implementations for every type that implements the trait, and there could                                                                                171 implementations for every type that implements the trait, and there could
172 theoretically be an infinite number of types.                                                                                                            172 theoretically be an infinite number of types.
173                                                                                                                                                          173 
174 For example, with:                                                                                                                                       174 For example, with:
175                                                                                                                                                          175 
176 ```                                                                                                                                                      176 ```
177 trait Trait {                                                                                                                                            177 trait Trait {
178     fn foo<T>(&self, on: T);                                                                                                                             178     fn foo<T>(&self, on: T);
179     // more methods                                                                                                                                      179     // more methods
180 }                                                                                                                                                        180 }
181                                                                                                                                                          181 
182 impl Trait for String {                                                                                                                                  182 impl Trait for String {
183     fn foo<T>(&self, on: T) {                                                                                                                            183     fn foo<T>(&self, on: T) {
184         // implementation 1                                                                                                                              184         // implementation 1
185     }                                                                                                                                                    185     }
186 }                                                                                                                                                        186 }
187                                                                                                                                                          187 
188 impl Trait for u8 {                                                                                                                                      188 impl Trait for u8 {
189     fn foo<T>(&self, on: T) {                                                                                                                            189     fn foo<T>(&self, on: T) {
190         // implementation 2                                                                                                                              190         // implementation 2
191     }                                                                                                                                                    191     }
192 }                                                                                                                                                        192 }
193                                                                                                                                                          193 
194 // 8 more implementations                                                                                                                                194 // 8 more implementations
195 ```                                                                                                                                                      195 ```
196                                                                                                                                                          196 
197 Now, if we have the following code:                                                                                                                      197 Now, if we have the following code:
198                                                                                                                                                          198 
199 ```ignore                                                                                                                                                199 ```ignore
200 fn call_foo(thing: Box<Trait>) {                                                                                                                         200 fn call_foo(thing: Box<Trait>) {
201     thing.foo(true); // this could be any one of the 8 types above                                                                                       201     thing.foo(true); // this could be any one of the 8 types above
202     thing.foo(1);                                                                                                                                        202     thing.foo(1);
203     thing.foo("hello");                                                                                                                                  203     thing.foo("hello");
204 }                                                                                                                                                        204 }
205 ```                                                                                                                                                      205 ```
206                                                                                                                                                          206 
207 We don't just need to create a table of all implementations of all methods of                                                                            207 We don't just need to create a table of all implementations of all methods of
208 `Trait`, we need to create such a table, for each different type fed to                                                                                  208 `Trait`, we need to create such a table, for each different type fed to
209 `foo()`. In this case this turns out to be (10 types implementing `Trait`)*(3                                                                            209 `foo()`. In this case this turns out to be (10 types implementing `Trait`)*(3
210 types being fed to `foo()`) = 30 implementations!                                                                                                        210 types being fed to `foo()`) = 30 implementations!
211                                                                                                                                                          211 
212 With real world traits these numbers can grow drastically.                                                                                               212 With real world traits these numbers can grow drastically.
213                                                                                                                                                          213 
214 To fix this, it is suggested to use a `where Self: Sized` bound similar to the                                                                           214 To fix this, it is suggested to use a `where Self: Sized` bound similar to the
215 fix for the sub-error above if you do not intend to call the method with type                                                                            215 fix for the sub-error above if you do not intend to call the method with type
216 parameters:                                                                                                                                              216 parameters:
217                                                                                                                                                          217 
218 ```                                                                                                                                                      218 ```
219 trait Trait {                                                                                                                                            219 trait Trait {
220     fn foo<T>(&self, on: T) where Self: Sized;                                                                                                           220     fn foo<T>(&self, on: T) where Self: Sized;
221     // more methods                                                                                                                                      221     // more methods
222 }                                                                                                                                                        222 }
223 ```                                                                                                                                                      223 ```
224                                                                                                                                                          224 
225 If this is not an option, consider replacing the type parameter with another                                                                             225 If this is not an option, consider replacing the type parameter with another
226 trait object (e.g. if `T: OtherTrait`, use `on: Box<OtherTrait>`). If the number                                                                         226 trait object (e.g. if `T: OtherTrait`, use `on: Box<OtherTrait>`). If the number
227 of types you intend to feed to this method is limited, consider manually listing                                                                         227 of types you intend to feed to this method is limited, consider manually listing
228 out the methods of different types.                                                                                                                      228 out the methods of different types.
229                                                                                                                                                          229 
230 ### Method has no receiver                                                                                                                               230 ### Method has no receiver
231                                                                                                                                                          231 
232 Methods that do not take a `self` parameter can't be called since there won't be                                                                         232 Methods that do not take a `self` parameter can't be called since there won't be
233 a way to get a pointer to the method table for them.                                                                                                     233 a way to get a pointer to the method table for them.
234                                                                                                                                                          234 
235 ```                                                                                                                                                      235 ```
236 trait Foo {                                                                                                                                              236 trait Foo {
237     fn foo() -> u8;                                                                                                                                      237     fn foo() -> u8;
238 }                                                                                                                                                        238 }
239 ```                                                                                                                                                      239 ```
240                                                                                                                                                          240 
241 This could be called as `<Foo as Foo>::foo()`, which would not be able to pick                                                                           241 This could be called as `<Foo as Foo>::foo()`, which would not be able to pick
242 an implementation.                                                                                                                                       242 an implementation.
243                                                                                                                                                          243 
244 Adding a `Self: Sized` bound to these methods will generally make this compile.                                                                          244 Adding a `Self: Sized` bound to these methods will generally make this compile.
245                                                                                                                                                          245 
246 ```                                                                                                                                                      246 ```
247 trait Foo {                                                                                                                                              247 trait Foo {
248     fn foo() -> u8 where Self: Sized;                                                                                                                    248     fn foo() -> u8 where Self: Sized;
249 }                                                                                                                                                        249 }
250 ```                                                                                                                                                      250 ```
251                                                                                                                                                          251 
252 ### The trait cannot use `Self` as a type parameter in the supertrait listing                                                                            252 ### The trait cannot use `Self` as a type parameter in the supertrait listing
253                                                                                                                                                          253 
254 This is similar to the second sub-error, but subtler. It happens in situations                                                                           254 This is similar to the second sub-error, but subtler. It happens in situations
255 like the following:                                                                                                                                      255 like the following:
256                                                                                                                                                          256 
257 ```compile_fail                                                                                                                                          257 ```compile_fail
258 trait Super<A> {}                                                                                                                                        258 trait Super<A> {}
259                                                                                                                                                          259 
260 trait Trait: Super<Self> {                                                                                                                               260 trait Trait: Super<Self> {
261 }                                                                                                                                                        261 }
262                                                                                                                                                          262 
263 struct Foo;                                                                                                                                              263 struct Foo;
264                                                                                                                                                          264 
265 impl Super<Foo> for Foo{}                                                                                                                                265 impl Super<Foo> for Foo{}
266                                                                                                                                                          266 
267 impl Trait for Foo {}                                                                                                                                    267 impl Trait for Foo {}
268 ```                                                                                                                                                      268 ```
269                                                                                                                                                          269 
270 Here, the supertrait might have methods as follows:                                                                                                      270 Here, the supertrait might have methods as follows:
271                                                                                                                                                          271 
272 ```                                                                                                                                                      272 ```
273 trait Super<A> {                                                                                                                                         273 trait Super<A> {
274     fn get_a(&self) -> A; // note that this is object safe!                                                                                              274     fn get_a(&self) -> A; // note that this is object safe!
275 }                                                                                                                                                        275 }
276 ```                                                                                                                                                      276 ```
277                                                                                                                                                          277 
278 If the trait `Foo` was deriving from something like `Super<String>` or                                                                                   278 If the trait `Foo` was deriving from something like `Super<String>` or
279 `Super<T>` (where `Foo` itself is `Foo<T>`), this is okay, because given a type                                                                          279 `Super<T>` (where `Foo` itself is `Foo<T>`), this is okay, because given a type
280 `get_a()` will definitely return an object of that type.                                                                                                 280 `get_a()` will definitely return an object of that type.
281                                                                                                                                                          281 
282 However, if it derives from `Super<Self>`, even though `Super` is object safe,                                                                           282 However, if it derives from `Super<Self>`, even though `Super` is object safe,
283 the method `get_a()` would return an object of unknown type when called on the                                                                           283 the method `get_a()` would return an object of unknown type when called on the
284 function. `Self` type parameters let us make object safe traits no longer safe,                                                                          284 function. `Self` type parameters let us make object safe traits no longer safe,
285 so they are forbidden when specifying supertraits.                                                                                                       285 so they are forbidden when specifying supertraits.
286                                                                                                                                                          286 
287 There's no easy fix for this, generally code will need to be refactored so that                                                                          287 There's no easy fix for this, generally code will need to be refactored so that
288 you no longer need to derive from `Super<Self>`.                                                                                                         288 you no longer need to derive from `Super<Self>`.
289 "##,                                                                                                                                                     289 "##,

