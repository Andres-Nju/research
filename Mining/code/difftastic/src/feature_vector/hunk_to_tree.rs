use crate::{
    parse::syntax::{Syntax, MatchedPos, MatchKind, get_novel_nodes},
    display::hunks::{Hunk},
    lines::LineNumber, positions::SingleLineSpan,
};
use rustc_hash::FxHashMap;
use tree_sitter as ts;
use ts::Node;


pub fn get_novels_from_hunk<'a>(lhs_positions: &'a Vec<MatchedPos>, rhs_positions: &'a Vec<MatchedPos>, hunk: &Hunk) -> (FxHashMap<LineNumber, Vec<&'a MatchedPos>>, FxHashMap<LineNumber, Vec<&'a MatchedPos>>){
    let mut lhs_novels = FxHashMap::default();
    for (_, lhs_line) in hunk.novel_lhs.iter().enumerate(){
        let lhs_line_nodes = get_novel_nodes(lhs_positions, lhs_line);
        lhs_novels.insert((*lhs_line).clone(), lhs_line_nodes);
    }

    let mut rhs_novels = FxHashMap::default();
    for (_, rhs_line) in hunk.novel_rhs.iter().enumerate(){
        let rhs_line_nodes = get_novel_nodes(rhs_positions, rhs_line);
        rhs_novels.insert((*rhs_line).clone(), rhs_line_nodes);
    }

    (lhs_novels, rhs_novels)
}

// 从Syntax树中找到与指定matchedpos对应的节点 
pub fn matched_pos_to_syntax<'a>(matched_pos: &MatchedPos, syntax_vec:&Vec<&'a Syntax<'a>>) -> Option<&'a Syntax<'a>>{
    for (_, syntax_ref) in syntax_vec.iter().enumerate(){
        match *syntax_ref{
            Syntax::List { 
                info, 
                open_position, 
                open_content, 
                children, 
                .. } =>{
                    match matched_pos_to_syntax(matched_pos, children){
                        Some(s) => {return Some(s);}
                        None => {}
                    }
                }
            Syntax::Atom { 
                info, 
                position, 
                content, 
                .. } => {
                    if is_inside_span(matched_pos.pos, position) {
                        return Some(*syntax_ref);
                    }
                }
        }
    }
    None
}

// 根据Syntax节点获取对应的tree-sitter::tree节点的cursor
pub fn syntax_to_tree_node<'a>(syntax: &'a Syntax<'a>, cursor: &mut ts::TreeCursor<'a>) -> Option<ts::TreeCursor<'a>>{
    match syntax{
        Syntax::Atom { 
            info, 
            position, 
            content, 
            .. } => {
                loop {
                    let node = cursor.node();
                    if is_the_same_position(&node, syntax) {
                        return Some(cursor.clone());
                    }
                    if cursor.goto_first_child() {
                        match syntax_to_tree_node(syntax, cursor){
                            Some(n) => {return Some(n);}
                            None => {}
                        }
                        cursor.goto_parent();
                    }
            
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        _ =>{}
    }
    None
}

// 从Tree中找到与指定matchedpos对应的节点 
pub fn matched_pos_to_tree_node<'a>(matched_pos: &MatchedPos, cursor: &mut ts::TreeCursor<'a>) -> Option<ts::TreeCursor<'a>>{
    let node = cursor.node();
    if (node.kind().contains("literal")){
        if is_inside_node_span(matched_pos.pos, &node){
            return Some(cursor.clone());
        }
    }
    if (node.child_count() == 0){
        //println!("{:?}", matched_pos);
        //println!("{:?}", node);
        if is_inside_node_span(matched_pos.pos, &node){
            return Some(cursor.clone());
        }
        else {
            return None;
        }
    }
    else {
        for c in node.children(cursor){
            match matched_pos_to_tree_node(matched_pos, &mut c.walk()){
                Some(n) => {return Some(n);}
                None => {}
            }
        }
        None
    }
}

// 判断一个matchedpos的line span是否包含在一个syntax node对应的line span中（一个syntax node可能跨行，而matched pos不会）
fn is_inside_span(single_span: SingleLineSpan, spans: &Vec<SingleLineSpan>) -> bool{
    if spans.len() == 1{ // syntax node 没有跨行，那么只要判断两个line span是否相同
        return single_span == spans[0];
    }
    for (_, span) in spans.iter().enumerate(){
        if single_span == *span {
            return true;
        }
    }
    return false;
}

// 判断一个matchedpos的line span是否包含在一个tree node对应的line span中（一个tree node可能跨行，而matched pos不会）
fn is_inside_node_span(single_span: SingleLineSpan, node: &ts::Node) -> bool{
    let node_start_pos = node.start_position();
    let node_end_pos = node.end_position();
    if (node_start_pos.row <= single_span.line.0 as usize && node_end_pos.row >= single_span.line.0 as usize){
        if (node_start_pos.row == node_end_pos.row) { // tree node 不跨行
            return node_start_pos.column <= single_span.start_col as usize && node_end_pos.column >= single_span.end_col as usize;
        }
        // tree node 跨行
        if ( node_start_pos.row == single_span.line.0 as usize) { // matched pos在node的起始行
            return node_start_pos.column <= single_span.start_col as usize;
        }
        if ( node_end_pos.row == single_span.line.0 as usize ){ // matched pos在node的结束行
            return node_end_pos.column >= single_span.end_col as usize;
        }
        return true;
    }
    return false;
}
// 判断一个tree-sitter node和syntax node是否是同一个（位置的）
fn is_the_same_position(node: &Node, syntax: &Syntax) -> bool{
    match syntax{
        Syntax::Atom { 
            info, 
            position, 
            .. } => {
                let node_start_pos = node.start_position();
                let node_end_pos = node.end_position();
                return node_start_pos.row == position[0].line.0 as usize && node_start_pos.column == position[0].start_col as usize &&
                node_end_pos.row == position[position.len() - 1].line.0 as usize && node_end_pos.column == position[position.len() - 1].end_col as usize;
            }
        _ =>{}
    }
    false
}