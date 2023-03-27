use std::env;
use std::fs;
//use difftastic::parse::tree_sitter_parse;
mod tree;

fn main() {
    // resolve the args
    let args: Vec<String> = env::args().collect();
    let method_path = &args[1][..];
    let ast_path = &args[2][..];

    let mut rust_parser = tree::create_rust_parser(); // get a new rust parser


    let res = fs::read_to_string(method_path);
    match res {
        Ok(source_code) => {
            // source_code为读入的method代码
            let option_tree = rust_parser.parse(&source_code[..], None);
            match option_tree {
                Some(ast_tree) => {
                    tree::write_tree_to_file(ast_path, &source_code[..], &ast_tree);
                },
                None => {
                    println!("parse file {} failed", method_path);
                    return;
                }
            };
        },
        Err(error) => {
            println!("open file {} failed", method_path);
            return;
        }
    };
    
}