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
                    if isInsideSpan(matched_pos.pos, position) {
                        return Some(*syntax_ref);
                    }
                }
        }
    }
    None
}

// 判断一个matchedpos的line span是否包含在一个syntax node对应的line span中（一个syntax node可能跨行，而matched pos不会）
fn isInsideSpan(single_span: SingleLineSpan, spans: &Vec<SingleLineSpan>) -> bool{
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