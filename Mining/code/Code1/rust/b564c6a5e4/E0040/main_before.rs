fn main() {
    let mut x = Foo { x: -7 };
    x.drop(); //~ ERROR E0040
}
