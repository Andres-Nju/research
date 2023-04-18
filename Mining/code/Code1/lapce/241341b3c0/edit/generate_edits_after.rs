pub fn generate_edits(
    old_text: &Rope,
    delta: &RopeDelta,
    edits: &mut Vec<tree_sitter::InputEdit>,
) {
    let (interval, _) = delta.summary();
    let (start, end) = interval.start_end();
    if let Some(inserted) = delta.as_simple_insert() {
        edits.push(create_insert_edit(old_text, start, inserted));
    } else if delta.is_simple_delete() {
        edits.push(create_delete_edit(old_text, start, end));
    } else {
        // TODO: This probably generates more insertions/deletions than it needs to.
        // It also creates a bunch of deltas and intermediate ropes which are not truly needed
        // Which is why, for the common case of simple inserts/deletions, we use the above logic

        // Break the delta into two parts, the insertions and the deletions
        // This makes it easier to translate them into the tree_sitter::InputEdit format
        let (insertions, deletions) = delta.clone().factor();

        let mut text = old_text.clone();
        for insert in InsertsValueIter::new(&insertions) {
            // We may not need the inserted text in order to calculate the new end position
            // but I was sufficiently uncertain, and so continued with how we did it previously
            let start = insert.old_offset;
            let inserted = insert.node;
            edits.push(create_insert_edit(&text, start, inserted));

            // Create a delta of this specific part of the insert
            // We have to apply it because future inserts assume it already happened
            let insert_delta = RopeDelta::simple_edit(
                Interval::new(start, start),
                inserted.clone(),
                text.len(),
            );
            text = insert_delta.apply(&text);
        }

        // We have to keep track of a shift because the deletions aren't properly moved forward
        let mut shift = insertions.inserts_len();
        for (start, end) in deletions.range_iter(CountMatcher::NonZero) {
            edits.push(create_delete_edit(&text, start + shift, end + shift));

            let delete_delta = RopeDelta::simple_edit(
                Interval::new(start + shift, end + shift),
                Rope::default(),
                text.len(),
            );
            text = delete_delta.apply(&text);
            shift -= end - start;
        }
    }
}
