
use tree_sitter::{Parser, Language};

extern "C" { fn tree_sitter_rust() -> Language; }
fn main() {

    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_rust() };
    
    parser.set_language(language).unwrap();

    let source_code = 
"fn main(){
    let b = 1;

    let s = String::from(\"23\");
    if (b >= 0){
        b = 1;
    }
}";
    let tree = parser.parse(source_code, None).unwrap();
    let root_node = tree.root_node();

    println!("{:?}", tree);
    // assert_eq!(root_node.kind(), "source_file");
    // assert_eq!(root_node.start_position().column, 0);
    // assert_eq!(root_node.end_position().column, 12);
}
