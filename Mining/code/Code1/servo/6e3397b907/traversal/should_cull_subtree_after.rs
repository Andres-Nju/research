    fn should_cull_subtree(
        &self,
        context: &mut StyleContext<E>,
        parent: E,
        parent_data: &ElementData,
    ) -> bool {
        debug_assert!(cfg!(feature = "gecko") ||
                      parent.has_current_styles_for_traversal(parent_data, context.shared.traversal_flags));

        // If the parent computed display:none, we don't style the subtree.
        if parent_data.styles.is_display_none() {
            debug!("Parent {:?} is display:none, culling traversal", parent);
            return true;
        }

        // Gecko-only XBL handling.
        //
        // If we're computing initial styles and the parent has a Gecko XBL
        // binding, that binding may inject anonymous children and remap the
        // explicit children to an insertion point (or hide them entirely). It
        // may also specify a scoped stylesheet, which changes the rules that
        // apply within the subtree. These two effects can invalidate the result
        // of property inheritance and selector matching (respectively) within
        // the subtree.
        //
        // To avoid wasting work, we defer initial styling of XBL subtrees until
        // frame construction, which does an explicit traversal of the unstyled
        // children after shuffling the subtree. That explicit traversal may in
        // turn find other bound elements, which get handled in the same way.
        //
        // We explicitly avoid handling restyles here (explicitly removing or
        // changing bindings), since that adds complexity and is rarer. If it
        // happens, we may just end up doing wasted work, since Gecko
        // recursively drops Servo ElementData when the XBL insertion parent of
        // an Element is changed.
        if cfg!(feature = "gecko") && context.thread_local.is_initial_style() &&
            parent_data.styles.primary().has_moz_binding()
        {
            debug!("Parent {:?} has XBL binding, deferring traversal", parent);
            return true;
        }

        return false;
    }

