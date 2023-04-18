fn main() {
    let strs = include_str!("words.txt").lines().map(|l| l.trim()).collect::<Vec<_>>();
    gen("js_word", "JsWord", &strs);
}
