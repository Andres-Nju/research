use std::fs;
use std::io::Read;
use tree_sitter::{Parser, Language, Tree, TreeCursor};


extern "C" { fn tree_sitter_rust() -> Language; }

pub fn create_rust_parser() ->Parser{
    // create a new parser
    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_rust() };
        
    parser.set_language(language).unwrap();
    return parser;
}


pub fn print_tree(src: &str, tree: &tree_sitter::Tree) {
    let mut cursor = tree.walk();
    print_cursor(src, &mut cursor, 0);
}

fn print_cursor(src: &str, cursor: &mut tree_sitter::TreeCursor, depth: usize) {
    loop {
        let node = cursor.node();
        node.end_position();

        let formatted_node = format!(
            "{} {} - {}",
            node.kind().replace('\n', "\\n"),
            node.start_position(),
            node.end_position()
        );

        if node.child_count() == 0 {
            let node_src = &src[node.start_byte()..node.end_byte()];
            println!("{}{} {:?}", "  ".repeat(depth), formatted_node, node_src);
        } else {
            println!("{}{}", "  ".repeat(depth), formatted_node,);
        }

        if cursor.goto_first_child() {
            print_cursor(src, cursor, depth + 1);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}


