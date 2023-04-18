File_Code/rust/af153cef0c/diagnostics/diagnostics_after.rs --- Rust
765 E0070: r##"                                                                                                                                              765 E0070: r##"
766 The left-hand side of an assignment operator must be a place expression. An                                                                              766 The left-hand side of an assignment operator must be a place expression. A
767 place expression represents a memory location and can be a variable (with                                                                                767 place expression represents a memory location and can be a variable (with
768 optional namespacing), a dereference, an indexing expression or a field                                                                                  768 optional namespacing), a dereference, an indexing expression or a field
769 reference.                                                                                                                                               769 reference.
770                                                                                                                                                          770 
771 More details can be found in the [Expressions] section of the Reference.                                                                                 771 More details can be found in the [Expressions] section of the Reference.
772                                                                                                                                                          772 
773 [Expressions]: https://doc.rust-lang.org/reference/expressions.html#places-rvalues-and-temporaries                                                       773 [Expressions]: https://doc.rust-lang.org/reference/expressions.html#places-rvalues-and-temporaries
774                                                                                                                                                          774 
775 Now, we can go further. Here are some erroneous code examples:                                                                                           775 Now, we can go further. Here are some erroneous code examples:
776                                                                                                                                                          776 
777 ```compile_fail,E0070                                                                                                                                    777 ```compile_fail,E0070
778 struct SomeStruct {                                                                                                                                      778 struct SomeStruct {
779     x: i32,                                                                                                                                              779     x: i32,
780     y: i32                                                                                                                                               780     y: i32
781 }                                                                                                                                                        781 }
782                                                                                                                                                          782 
783 const SOME_CONST : i32 = 12;                                                                                                                             783 const SOME_CONST : i32 = 12;
784                                                                                                                                                          784 
785 fn some_other_func() {}                                                                                                                                  785 fn some_other_func() {}
786                                                                                                                                                          786 
787 fn some_function() {                                                                                                                                     787 fn some_function() {
788     SOME_CONST = 14; // error : a constant value cannot be changed!                                                                                      788     SOME_CONST = 14; // error : a constant value cannot be changed!
789     1 = 3; // error : 1 isn't a valid place!                                                                                                             789     1 = 3; // error : 1 isn't a valid place!
790     some_other_func() = 4; // error : we can't assign value to a function!                                                                               790     some_other_func() = 4; // error : we can't assign value to a function!
791     SomeStruct.x = 12; // error : SomeStruct a structure name but it is used                                                                             791     SomeStruct.x = 12; // error : SomeStruct a structure name but it is used
792                        // like a variable!                                                                                                               792                        // like a variable!
793 }                                                                                                                                                        793 }
794 ```                                                                                                                                                      794 ```
795                                                                                                                                                          795 
796 And now let's give working examples:                                                                                                                     796 And now let's give working examples:
797                                                                                                                                                          797 
798 ```                                                                                                                                                      798 ```
799 struct SomeStruct {                                                                                                                                      799 struct SomeStruct {
800     x: i32,                                                                                                                                              800     x: i32,
801     y: i32                                                                                                                                               801     y: i32
802 }                                                                                                                                                        802 }
803 let mut s = SomeStruct {x: 0, y: 0};                                                                                                                     803 let mut s = SomeStruct {x: 0, y: 0};
804                                                                                                                                                          804 
805 s.x = 3; // that's good !                                                                                                                                805 s.x = 3; // that's good !
806                                                                                                                                                          806 
807 // ...                                                                                                                                                   807 // ...
808                                                                                                                                                          808 
809 fn some_func(x: &mut i32) {                                                                                                                              809 fn some_func(x: &mut i32) {
810     *x = 12; // that's good !                                                                                                                            810     *x = 12; // that's good !
811 }                                                                                                                                                        811 }
812 ```                                                                                                                                                      812 ```
813 "##,                                                                                                                                                     813 "##,

