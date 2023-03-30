use crate::{
    parse::syntax::{Syntax, MatchedPos, MatchKind, get_novel_nodes},
    display::hunks::{Hunk},
    lines::LineNumber,
};
use rustc_hash::FxHashMap;
use tree_sitter as ts;
use ts::Node;


pub fn get_novels_from_hunk<'a>(positions: &'a Vec<MatchedPos>, hunk: &Hunk) -> (FxHashMap<LineNumber, Vec<&'a MatchedPos>>, FxHashMap<LineNumber, Vec<&'a MatchedPos>>){
    let mut lhs_novels = FxHashMap::default();
    for (_, lhs_line) in hunk.novel_lhs.iter().enumerate(){
        let lhs_line_nodes = get_novel_nodes(positions, lhs_line);
        lhs_novels.insert((*lhs_line).clone(), lhs_line_nodes);
    }

    let mut rhs_novels = FxHashMap::default();
    for (_, rhs_line) in hunk.novel_rhs.iter().enumerate(){
        let rhs_line_nodes = get_novel_nodes(positions, rhs_line);
        rhs_novels.insert((*rhs_line).clone(), rhs_line_nodes);
    }

    (lhs_novels, rhs_novels)
}