File_Code/rust/16547f5025/error_codes/error_codes_after.rs --- Rust
555 E0666: r##"                                                                                                                                              555 E0666: r##"
556 `impl Trait` types cannot appear nested in the                                                                                                           556 `impl Trait` types cannot appear nested in the
557 generic types of other `impl Trait` types.                                                                                                               557 generic arguments of other `impl Trait` types.
558                                                                                                                                                          558 
559 Example of erroneous code:                                                                                                                               559 Example of erroneous code:
560                                                                                                                                                          560 
561 ```compile_fail,E0666                                                                                                                                    561 ```compile_fail,E0666
562 trait MyGenericTrait<T> {}                                                                                                                               562 trait MyGenericTrait<T> {}
563 trait MyInnerTrait {}                                                                                                                                    563 trait MyInnerTrait {}
564                                                                                                                                                          564 
565 fn foo(bar: impl MyGenericTrait<impl MyInnerTrait>) {}                                                                                                   565 fn foo(bar: impl MyGenericTrait<impl MyInnerTrait>) {}
566 ```                                                                                                                                                      566 ```
567                                                                                                                                                          567 
568 Type parameters for `impl Trait` types must be                                                                                                           568 Type parameters for `impl Trait` types must be
569 explicitly defined as named generic parameters:                                                                                                          569 explicitly defined as named generic parameters:
570                                                                                                                                                          570 
571 ```                                                                                                                                                      571 ```
572 trait MyGenericTrait<T> {}                                                                                                                               572 trait MyGenericTrait<T> {}
573 trait MyInnerTrait {}                                                                                                                                    573 trait MyInnerTrait {}
574                                                                                                                                                          574 
575 fn foo<T: MyInnerTrait>(bar: impl MyGenericTrait<T>) {}                                                                                                  575 fn foo<T: MyInnerTrait>(bar: impl MyGenericTrait<T>) {}
576 ```                                                                                                                                                      576 ```
577 "##,                                                                                                                                                     577 "##,

