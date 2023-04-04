use std::collections::HashMap;

use crate::{
    parse::syntax::{Syntax, MatchedPos, MatchKind, get_novel_nodes},
    display::hunks::{Hunk},
    lines::LineNumber, positions::SingleLineSpan,
};
use crossterm::cursor;
use rustc_hash::FxHashMap;
use tree_edit_distance::Tree;
use tree_sitter as ts;
use ts::{Node, TreeCursor};

#[derive(Clone)]
pub enum ChangeType{
    Added,
    Deleted,
    MaybeUpdated,
    DeletedThenAdded
}

pub fn tree_to_edit_action<'a> (lhs_root: &TreeCursor<'a>, lhs_src: &str, rhs_root: &TreeCursor<'a>, rhs_src: &str) -> (Vec<TreeCursor<'a>>, Vec<TreeCursor<'a>>, Vec<(TreeCursor<'a>, TreeCursor<'a>)>){
    let mut added = vec![];
    let mut deleted = vec![];
    let mut updated = vec![];
    // let mut updated = vec![];

    let mut lhs_child_num = lhs_root.node().child_count();
    let mut rhs_child_num = rhs_root.node().child_count();
    if (lhs_child_num == 0 && rhs_child_num == 0){
        // println!("****\n{:?}, {:?}\n*****", lhs_root.node(), rhs_root.node());
        if (lhs_src[lhs_root.node().start_byte()..lhs_root.node().end_byte()] != rhs_src[rhs_root.node().start_byte()..rhs_root.node().end_byte()]){
            updated.push((lhs_root.clone(), rhs_root.clone()));
        }
    }
    else if lhs_child_num == 0{
        added.push(rhs_root.clone());
    }
    else if rhs_child_num == 0{
        deleted.push(lhs_root.clone());
    }
    else if lhs_root.node().kind() == "string_literal" && rhs_root.node().kind() == "string_literal"{
        if (lhs_src[lhs_root.node().start_byte()..lhs_root.node().end_byte()] != rhs_src[rhs_root.node().start_byte()..rhs_root.node().end_byte()]){
            updated.push((lhs_root.clone(), rhs_root.clone()));
        }
    }
    else {
        let mut lhs_nodes = vec![];
        let mut rhs_nodes = vec![];
        let mut lhs_cursor = lhs_root.clone();
        let mut rhs_cursor = rhs_root.clone();
        lhs_cursor.goto_first_child();
        rhs_cursor.goto_first_child();
        loop {
            lhs_nodes.push(lhs_cursor.clone());
            if !lhs_cursor.goto_next_sibling(){
                break;
            }
        }
        loop {
            rhs_nodes.push(rhs_cursor.clone());
            if !rhs_cursor.goto_next_sibling() {
                break;
            }
        }
        let (children_added, children_deleted, children_maybe_updated) = get_node_change_type(&lhs_nodes, &rhs_nodes, &calculate_edit_action(&lhs_nodes, &rhs_nodes));
        added.extend(children_added);
        deleted.extend(children_deleted);
        for (_, (lhs_child_root, rhs_child_root)) in children_maybe_updated.iter().enumerate(){
            let (new_added, new_deleted, new_updated) = tree_to_edit_action(lhs_child_root, lhs_src, rhs_child_root, rhs_src);
            added.extend(new_added);
            deleted.extend(new_deleted);
            updated.extend(new_updated);
        }
    }


    (added, deleted, updated)
}
// 对两棵树上同一层的节点， 根据其edit path确定具体的change type
pub fn get_node_change_type<'a> (nodes_1: &Vec<TreeCursor<'a>>, nodes_2: &Vec<TreeCursor<'a>>, path: &Vec<Vec<ChangeType>>)-> (Vec<TreeCursor<'a>>, Vec<TreeCursor<'a>>, Vec<(TreeCursor<'a>, TreeCursor<'a>)>){
    let mut added = vec![];
    let mut deleted = vec![];
    let mut maybe_updated = vec![];

    let mut i = nodes_1.len();
    let mut j = nodes_2.len();

    while i > 0 || j > 0{
        match path[i][j]{
            ChangeType::Added =>{
                j -= 1;
                added.push(nodes_2[j].clone());
            }
            ChangeType::Deleted =>{
                i -= 1;
                deleted.push(nodes_1[i].clone());
            }
            ChangeType::DeletedThenAdded =>{
                j -= 1;
                added.push(nodes_2[j].clone());
                i -= 1;
                deleted.push(nodes_1[i].clone());
            }
            ChangeType::MaybeUpdated =>{
                i -= 1;
                j -= 1;
                maybe_updated.push((nodes_1[i].clone(), nodes_2[j].clone()));
            }
        }
    }

    (added, deleted, maybe_updated)
}

fn calculate_edit_action<'a>(nodes_1: &Vec<TreeCursor>, nodes_2: &Vec<TreeCursor>) -> Vec<Vec<ChangeType>>{
    let n = nodes_1.len();
    let m = nodes_2.len();

    let mut cost = vec![vec![0; m + 1]; n + 1];
    let mut path = vec![vec![ChangeType::Deleted; m + 1]; n + 1];

    for j in 1..(m + 1){
        path[0][j] = ChangeType::Added;
    }
    for i in 1..(n + 1){
        for j in 1..(m + 1){
            if nodes_1[i - 1].node().kind() == nodes_2[j - 1].node().kind(){
                path[i][j] = ChangeType::MaybeUpdated;
                cost[i][j] = cost[i - 1][j - 1];
                // node_1[i - 1]的children 和 node_2[j - 1]的children进行递归比较
            }
            else if (cost[i][j - 1] <= cost[i - 1][j] && cost[i][j - 1] <= cost[i - 1][j - 1] + 1) {
                cost[i][j] = cost[i][j - 1] + 1;
                path[i][j] = ChangeType::Added;
                // nodes_2[j- 1]: add 
            }
            else if (cost[i - 1][j] <= cost[i][j - 1] && cost[i - 1][j] <= cost[i - 1][j - 1] + 1) {
                cost[i][j] = cost[i - 1][j] + 1;
                path[i][j] = ChangeType::Deleted;
                // nodes_1[i- 1]: delete 
            }
            else{
                cost[i][j] = cost[i - 1][j - 1] + 2;
                path[i][j] = ChangeType::DeletedThenAdded;
                // nodes_1[i- 1]: delete 
                // nodes_2[j- 1]: add 
            }
        }
    }
    path
}

fn is_same_label(cursor_1: &TreeCursor, cursor_2: &TreeCursor) -> bool {
    if cursor_1.node().kind() == cursor_2.node().kind(){
        if cursor_1.node().kind() == "block"{
            return true;
        }
        if (cursor_1.node().child_count() == cursor_2.node().child_count()){
            if cursor_1.node().child_count() == 0{
                return true;
            }
            let mut cursor1 = cursor_1.clone();
            let mut cursor2 = cursor_2.clone();
            cursor1.goto_first_child();
            cursor2.goto_first_child();
            loop {
                if cursor1.node().kind() != cursor2.node().kind(){
                    return false;
                }

                if !cursor1.goto_next_sibling(){
                    break;
                }
                if !cursor2.goto_next_sibling(){
                    break;
                }
            }
            return true;
        }
    }

    false
}