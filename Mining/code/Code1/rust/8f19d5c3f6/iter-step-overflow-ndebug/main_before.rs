fn main() {
    let mut it = u8::max_value()..;
    assert_eq!(it.next().unwrap(), u8::min_value());

    let mut it = i8::max_value()..;
    assert_eq!(it.next().unwrap(), i8::min_value());
}
