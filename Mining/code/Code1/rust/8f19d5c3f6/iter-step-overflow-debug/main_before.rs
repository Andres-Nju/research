fn main() {
    let r = panic::catch_unwind(|| {
        let mut it = u8::max_value()..;
        it.next().unwrap();
    });
    assert!(r.is_err());

    let r = panic::catch_unwind(|| {
        let mut it = i8::max_value()..;
        it.next().unwrap();
    });
    assert!(r.is_err());
}
