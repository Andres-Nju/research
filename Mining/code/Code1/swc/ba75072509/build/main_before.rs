fn main() {
    let strs = include_str!("words.txt").split("\n").collect::<Vec<_>>();
    gen("js_word", "JsWord", &strs);
}
