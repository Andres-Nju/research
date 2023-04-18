fn main() {
    id!(x?);  //~ error: the `?` operator is not stable (see issue #31436)
    //~^ help: add #![feature(question_mark)] to the crate attributes to enable
    y?;  //~ error: the `?` operator is not stable (see issue #31436)
    //~^ help: add #![feature(question_mark)] to the crate attributes to enable
}
