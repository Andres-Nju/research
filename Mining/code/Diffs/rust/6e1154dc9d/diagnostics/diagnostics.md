File_Code/rust/6e1154dc9d/diagnostics/diagnostics_after.rs --- Rust
15 E0023: r##"                                                                                                                                               15 E0023: r##"
16 A pattern used to match against an enum variant must provide a sub-pattern for                                                                            16 A pattern used to match against an enum variant must provide a sub-pattern for
17 each field of the enum variant. This error indicates that a pattern attempted to                                                                          17 each field of the enum variant. This error indicates that a pattern attempted to
18 extract an incorrect number of fields from a variant.                                                                                                     18 extract an incorrect number of fields from a variant.
19                                                                                                                                                           19 
20 ```                                                                                                                                                       20 ```
21 enum Fruit {                                                                                                                                              21 enum Fruit {
22     Apple(String, String),                                                                                                                                22     Apple(String, String),
23     Pear(u32),                                                                                                                                            23     Pear(u32),
24 }                                                                                                                                                         24 }
25 ```                                                                                                                                                       25 ```
26                                                                                                                                                           26 
27 Here the `Apple` variant has two fields, and should be matched against like so:                                                                           27 Here the `Apple` variant has two fields, and should be matched against like so:
28                                                                                                                                                           28 
29 ```                                                                                                                                                       29 ```
30 enum Fruit {                                                                                                                                              30 enum Fruit {
31     Apple(String, String),                                                                                                                                31     Apple(String, String),
32     Pear(u32),                                                                                                                                            32     Pear(u32),
33 }                                                                                                                                                         33 }
34                                                                                                                                                           34 
35 let x = Fruit::Apple(String::new(), String::new());                                                                                                       35 let x = Fruit::Apple(String::new(), String::new());
36                                                                                                                                                           36 
37 // Correct.                                                                                                                                               37 // Correct.
38 match x {                                                                                                                                                 38 match x {
39     Fruit::Apple(a, b) => {},                                                                                                                             39     Fruit::Apple(a, b) => {},
40     _ => {}                                                                                                                                               40     _ => {}
41 }                                                                                                                                                         41 }
42 ```                                                                                                                                                       42 ```
43                                                                                                                                                           43 
44 Matching with the wrong number of fields has no sensible interpretation:                                                                                  44 Matching with the wrong number of fields has no sensible interpretation:
45                                                                                                                                                           45 
46 ```compile_fail                                                                                                                                           46 ```compile_fail
47 enum Fruit {                                                                                                                                              47 enum Fruit {
48     Fruit::Apple(String, String),                                                                                                                         48     Apple(String, String),
49     Fruit::Pear(u32),                                                                                                                                     49     Pear(u32),
50 }                                                                                                                                                         50 }
51                                                                                                                                                           51 
52 let x = Fruit::Apple(String::new(), String::new());                                                                                                       52 let x = Fruit::Apple(String::new(), String::new());
53                                                                                                                                                           53 
54 // Incorrect.                                                                                                                                             54 // Incorrect.
55 match x {                                                                                                                                                 55 match x {
56     Apple(a) => {},                                                                                                                                       56     Fruit::Apple(a) => {},
57     Apple(a, b, c) => {},                                                                                                                                 57     Fruit::Apple(a, b, c) => {},
58 }                                                                                                                                                         58 }
59 ```                                                                                                                                                       59 ```
60                                                                                                                                                           60 
61 Check how many fields the enum was declared with and ensure that your pattern                                                                             61 Check how many fields the enum was declared with and ensure that your pattern
62 uses the same number.                                                                                                                                     62 uses the same number.
63 "##,                                                                                                                                                      63 "##,

