fn main() {
    let res: Option<bool> = try {
        true
    } catch { };
    //~^ ERROR `try {} catch` is not a valid syntax
}
