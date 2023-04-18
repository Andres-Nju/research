fn traverse_subtree(element: GeckoElement, raw_data: RawServoStyleSetBorrowed,
                    traversal_flags: TraversalFlags) {
    // When new content is inserted in a display:none subtree, we will call into
    // servo to try to style it. Detect that here and bail out.
    if let Some(parent) = element.parent_element() {
        if parent.borrow_data().map_or(true, |d| d.styles().is_display_none()) {
            debug!("{:?} has unstyled parent {:?} - ignoring call to traverse_subtree", element, parent);
            return;
        }
    }

    let per_doc_data = PerDocumentStyleData::from_ffi(raw_data).borrow();

    let token = RecalcStyleOnly::pre_traverse(element, &per_doc_data.stylist, traversal_flags);
    if !token.should_traverse() {
        return;
    }

    debug!("Traversing subtree:");
    debug!("{:?}", ShowSubtreeData(element.as_node()));

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let shared_style_context = create_shared_context(&global_style_data,
                                                     &guard,
                                                     &per_doc_data,
                                                     traversal_flags);

    let traversal_driver = if global_style_data.style_thread_pool.is_none() {
        TraversalDriver::Sequential
    } else {
        TraversalDriver::Parallel
    };

    let traversal = RecalcStyleOnly::new(shared_style_context, traversal_driver);
    if traversal_driver.is_parallel() {
        parallel::traverse_dom(&traversal, element, token,
                               global_style_data.style_thread_pool.as_ref().unwrap());
    } else {
        sequential::traverse_dom(&traversal, element, token);
    }
}
