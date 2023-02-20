fn main(){
    let source_code = "use std::path::PathBuf;


    use rust_code_analysis::{dump_node, RustParser, ParserTrait};
    
    fn main(){
    
        // The path to a dummy file used to contain the source code
        let path = PathBuf::from();
        let source_as_vec = source_code.as_bytes().to_vec();
    
        // The parser of the code, in this case a Rust parser
        let parser = RustParser::new(source_as_vec.clone(), &path, None);
    
        // The root of the AST
        let root = parser.get_root();
    
        // Dump the AST from the first line of code in a file to the last one
        dump_node(&source_as_vec, &root, -1, None, None).unwrap();
    }";

    // The path to a dummy file used to contain the source code
    let path = PathBuf::from("foo.rs");
    let source_as_vec = source_code.as_bytes().to_vec();

    // The parser of the code, in this case a Rust parser
    let parser = RustParser::new(source_as_vec.clone(), &path, None);

    // The root of the AST
    let root = parser.get_root();

    // Dump the AST from the first line of code in a file to the last one
    let res =dump_node(&source_as_vec, &root, -1, None, None).unwrap();
}
