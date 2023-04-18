    fn from_content(content: &'ln nsIContent) -> Self {
        GeckoNode(&content._base)
    }

    #[inline]
    fn flags(&self) -> u32 {
        (self.0)._base._base_1.mFlags
    }

    #[inline]
    fn node_info(&self) -> &structs::NodeInfo {
        debug_assert!(!self.0.mNodeInfo.mRawPtr.is_null());
        unsafe { &*self.0.mNodeInfo.mRawPtr }
    }

    // These live in different locations depending on processor architecture.
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn bool_flags(&self) -> u32 {
        (self.0)._base._base_1.mBoolFlags
    }

    #[cfg(target_pointer_width = "32")]
    #[inline]
    fn bool_flags(&self) -> u32 {
        (self.0).mBoolFlags
    }

    #[inline]
    fn get_bool_flag(&self, flag: nsINode_BooleanFlag) -> bool {
        self.bool_flags() & (1u32 << flag as u32) != 0
    }

    fn owner_doc(&self) -> &structs::nsIDocument {
        debug_assert!(!self.node_info().mDocument.is_null());
        unsafe { &*self.node_info().mDocument }
    }

    #[inline]
    fn first_child(&self) -> Option<GeckoNode<'ln>> {
        unsafe { self.0.mFirstChild.as_ref().map(GeckoNode::from_content) }
    }

    #[inline]
    fn last_child(&self) -> Option<GeckoNode<'ln>> {
        unsafe { Gecko_GetLastChild(self.0).map(GeckoNode) }
    }

    #[inline]
    fn prev_sibling(&self) -> Option<GeckoNode<'ln>> {
        unsafe { self.0.mPreviousSibling.as_ref().map(GeckoNode::from_content) }
    }

    #[inline]
    fn next_sibling(&self) -> Option<GeckoNode<'ln>> {
        unsafe { self.0.mNextSibling.as_ref().map(GeckoNode::from_content) }
    }

    /// Simple iterator over all this node's children.  Unlike `.children()`, this iterator does
    /// not filter out nodes that don't need layout.
    fn dom_children(self) -> GeckoChildrenIterator<'ln> {
        GeckoChildrenIterator::Current(self.first_child())
    }

    /// WARNING: This logic is duplicated in Gecko's FlattenedTreeParentIsParent.
    /// Make sure to mirror any modifications in both places.
    fn flattened_tree_parent_is_parent(&self) -> bool {
        use ::gecko_bindings::structs::*;
        let flags = self.flags();
        if flags & (NODE_MAY_BE_IN_BINDING_MNGR as u32 |
                    NODE_IS_IN_SHADOW_TREE as u32) != 0 {
            return false;
        }

        let parent = unsafe { self.0.mParent.as_ref() }.map(GeckoNode);
        let parent_el = parent.and_then(|p| p.as_element());
        if flags & (NODE_IS_NATIVE_ANONYMOUS_ROOT as u32) != 0 &&
           parent_el.map_or(false, |el| el.is_root())
        {
            return false;
        }

        if parent_el.map_or(false, |el| el.has_shadow_root()) {
            return false;
        }

        true
    }

    fn flattened_tree_parent(&self) -> Option<Self> {
        let fast_path = self.flattened_tree_parent_is_parent();
        debug_assert!(fast_path == unsafe { bindings::Gecko_FlattenedTreeParentIsParent(self.0) });
        if fast_path {
            unsafe { self.0.mParent.as_ref().map(GeckoNode) }
        } else {
            unsafe { bindings::Gecko_GetFlattenedTreeParentNode(self.0).map(GeckoNode) }
        }
    }

    /// This logic is duplicated in Gecko's nsIContent::IsRootOfNativeAnonymousSubtree.
    fn is_root_of_native_anonymous_subtree(&self) -> bool {
        use gecko_bindings::structs::NODE_IS_NATIVE_ANONYMOUS_ROOT;
        return self.flags() & (NODE_IS_NATIVE_ANONYMOUS_ROOT as u32) != 0
    }

    fn contains_non_whitespace_content(&self) -> bool {
        unsafe { Gecko_IsSignificantChild(self.0, true, false) }
    }

    #[inline]
    fn may_have_anonymous_children(&self) -> bool {
        self.get_bool_flag(nsINode_BooleanFlag::ElementMayHaveAnonymousChildren)
    }

    /// This logic is duplicated in Gecko's nsIContent::IsInAnonymousSubtree.
    #[inline]
    fn is_in_anonymous_subtree(&self) -> bool {
        use gecko_bindings::structs::NODE_IS_IN_SHADOW_TREE;
        self.flags() & (NODE_IS_IN_NATIVE_ANONYMOUS_SUBTREE as u32) != 0 ||
        ((self.flags() & (NODE_IS_IN_SHADOW_TREE as u32) == 0) &&
         self.as_element().map_or(false, |e| e.has_xbl_binding_parent()))
    }
}

impl<'ln> NodeInfo for GeckoNode<'ln> {
    #[inline]
    fn is_element(&self) -> bool {
        self.get_bool_flag(nsINode_BooleanFlag::NodeIsElement)
    }

    fn is_text_node(&self) -> bool {
        // This is a DOM constant that isn't going to change.
        const TEXT_NODE: u16 = 3;
        self.node_info().mInner.mNodeType == TEXT_NODE
    }
}

impl<'ln> TNode for GeckoNode<'ln> {
    type ConcreteElement = GeckoElement<'ln>;
    type ConcreteChildrenIterator = GeckoChildrenIterator<'ln>;

    fn to_unsafe(&self) -> UnsafeNode {
        (self.0 as *const _ as usize, 0)
    }

    unsafe fn from_unsafe(n: &UnsafeNode) -> Self {
        GeckoNode(&*(n.0 as *mut RawGeckoNode))
    }

    fn parent_node(&self) -> Option<Self> {
        unsafe { self.0.mParent.as_ref().map(GeckoNode) }
    }

    fn children(&self) -> LayoutIterator<GeckoChildrenIterator<'ln>> {
        LayoutIterator(self.dom_children())
    }

    fn traversal_parent(&self) -> Option<GeckoElement<'ln>> {
        self.flattened_tree_parent().and_then(|n| n.as_element())
    }

    fn traversal_children(&self) -> LayoutIterator<GeckoChildrenIterator<'ln>> {
        if let Some(element) = self.as_element() {
            // This condition is similar to the check that
            // StyleChildrenIterator::IsNeeded does, except that it might return
            // true if we used to (but no longer) have anonymous content from
            // ::before/::after, XBL bindings, or nsIAnonymousContentCreators.
            if self.is_in_anonymous_subtree() ||
               element.has_xbl_binding_with_content() ||
               self.may_have_anonymous_children() {
                unsafe {
                    let mut iter: structs::StyleChildrenIterator = ::std::mem::zeroed();
                    Gecko_ConstructStyleChildrenIterator(element.0, &mut iter);
                    return LayoutIterator(GeckoChildrenIterator::GeckoIterator(iter));
                }
            }
        }

        LayoutIterator(self.dom_children())
    }

    fn opaque(&self) -> OpaqueNode {
        let ptr: usize = self.0 as *const _ as usize;
        OpaqueNode(ptr)
    }

    fn debug_id(self) -> usize {
        unimplemented!()
    }

    fn as_element(&self) -> Option<GeckoElement<'ln>> {
        if self.is_element() {
            unsafe { Some(GeckoElement(&*(self.0 as *const _ as *const RawGeckoElement))) }
        } else {
            None
        }
    }

    fn can_be_fragmented(&self) -> bool {
        // FIXME(SimonSapin): Servo uses this to implement CSS multicol / fragmentation
        // Maybe this isn’t useful for Gecko?
        false
    }

    unsafe fn set_can_be_fragmented(&self, _value: bool) {
        // FIXME(SimonSapin): Servo uses this to implement CSS multicol / fragmentation
        // Maybe this isn’t useful for Gecko?
    }

    fn is_in_doc(&self) -> bool {
        unsafe { bindings::Gecko_IsInDocument(self.0) }
    }

    fn needs_dirty_on_viewport_size_changed(&self) -> bool {
        // Gecko's node doesn't have the DIRTY_ON_VIEWPORT_SIZE_CHANGE flag,
        // so we force them to be dirtied on viewport size change, regardless if
        // they use viewport percentage size or not.
        // TODO(shinglyu): implement this in Gecko: https://github.com/servo/servo/pull/11890
        true
    }

    // TODO(shinglyu): implement this in Gecko: https://github.com/servo/servo/pull/11890
    unsafe fn set_dirty_on_viewport_size_changed(&self) {}
}

/// A wrapper on top of two kind of iterators, depending on the parent being
/// iterated.
///
/// We generally iterate children by traversing the light-tree siblings of the
/// first child like Servo does.
///
/// However, for nodes with anonymous children, we use a custom (heavier-weight)
/// Gecko-implemented iterator.
///
/// FIXME(emilio): If we take into account shadow DOM, we're going to need the
/// flat tree pretty much always. We can try to optimize the case where there's
/// no shadow root sibling, probably.
pub enum GeckoChildrenIterator<'a> {
    /// A simple iterator that tracks the current node being iterated and
    /// replaces it with the next sibling when requested.
    Current(Option<GeckoNode<'a>>),
    /// A Gecko-implemented iterator we need to drop appropriately.
    GeckoIterator(structs::StyleChildrenIterator),
}

impl<'a> Drop for GeckoChildrenIterator<'a> {
    fn drop(&mut self) {
        if let GeckoChildrenIterator::GeckoIterator(ref mut it) = *self {
            unsafe {
                Gecko_DestroyStyleChildrenIterator(it);
            }
        }
    }
}

