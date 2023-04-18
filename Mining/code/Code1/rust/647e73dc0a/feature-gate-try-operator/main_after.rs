fn main() {
    id!(x?);  //~ error: the `?` operator is not stable (see issue #31436)
    y?;  //~ error: the `?` operator is not stable (see issue #31436)
}

