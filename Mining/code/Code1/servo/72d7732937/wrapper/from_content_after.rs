    fn from_content(content: &'ln nsIContent) -> Self {
        GeckoNode(&content._base)
    }

    fn node_info(&self) -> &structs::NodeInfo {
        debug_assert!(!self.0.mNodeInfo.mRawPtr.is_null());
        unsafe { &*self.0.mNodeInfo.mRawPtr }
    }

    fn flags(&self) -> u32 {
        (self.0)._base._base_1.mFlags
    }

    // FIXME: We can implement this without OOL calls, but we can't easily given
    // GeckoNode is a raw reference.
    //
    // We can use a Cell<T>, but that's a bit of a pain.
    fn set_flags(&self, flags: u32) {
        unsafe { Gecko_SetNodeFlags(self.0, flags) }
    }

    fn unset_flags(&self, flags: u32) {
        unsafe { Gecko_UnsetNodeFlags(self.0, flags) }
    }

    fn get_node_data(&self) -> Option<&NonOpaqueStyleData> {
        unsafe {
            from_opaque_style_data(self.0.mServoData.get()).as_ref()
        }
    }

    pub fn initialize_data(self) {
        if self.get_node_data().is_none() {
            let ptr = Box::new(NonOpaqueStyleData::new());
            debug_assert!(self.0.mServoData.get().is_null());
            self.0.mServoData.set(to_opaque_style_data(Box::into_raw(ptr)));
        }
    }

    pub fn clear_data(self) {
        if !self.get_node_data().is_none() {
            let d = from_opaque_style_data(self.0.mServoData.get());
            let _ = unsafe { Box::from_raw(d) };
            self.0.mServoData.set(ptr::null_mut());
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeckoRestyleDamage(nsChangeHint);

impl TRestyleDamage for GeckoRestyleDamage {
    type PreExistingComputedValues = nsStyleContext;

    fn empty() -> Self {
        use std::mem;
        GeckoRestyleDamage(unsafe { mem::transmute(0u32) })
    }

    fn compute(source: &nsStyleContext,
               new_style: &Arc<ComputedValues>) -> Self {
        let context = source as *const nsStyleContext as *mut nsStyleContext;
        let hint = unsafe { Gecko_CalcStyleDifference(context, new_style.as_borrowed_opt().unwrap()) };
        GeckoRestyleDamage(hint)
    }

    fn rebuild_and_reflow() -> Self {
        GeckoRestyleDamage(nsChangeHint::nsChangeHint_ReconstructFrame)
    }
}

impl BitOr for GeckoRestyleDamage {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        use std::mem;
        GeckoRestyleDamage(unsafe { mem::transmute(self.0 as u32 | other.0 as u32) })
    }
}


impl<'ln> NodeInfo for GeckoNode<'ln> {
    fn is_element(&self) -> bool {
        use gecko_bindings::structs::nsINode_BooleanFlag;
        self.0.mBoolFlags & (1u32 << nsINode_BooleanFlag::NodeIsElement as u32) != 0
    }

    fn is_text_node(&self) -> bool {
        // This is a DOM constant that isn't going to change.
        const TEXT_NODE: u16 = 3;
        self.node_info().mInner.mNodeType == TEXT_NODE
    }
}

impl<'ln> TNode for GeckoNode<'ln> {
    type ConcreteDocument = GeckoDocument<'ln>;
    type ConcreteElement = GeckoElement<'ln>;
    type ConcreteRestyleDamage = GeckoRestyleDamage;
    type ConcreteChildrenIterator = GeckoChildrenIterator<'ln>;

    fn to_unsafe(&self) -> UnsafeNode {
        (self.0 as *const _ as usize, 0)
    }

    unsafe fn from_unsafe(n: &UnsafeNode) -> Self {
        GeckoNode(&*(n.0 as *mut RawGeckoNode))
    }

    fn dump(self) {
        unimplemented!()
    }

    fn dump_style(self) {
        unimplemented!()
    }

    fn children(self) -> LayoutIterator<GeckoChildrenIterator<'ln>> {
        let maybe_iter = unsafe { Gecko_MaybeCreateStyleChildrenIterator(self.0) };
        if let Some(iter) = maybe_iter.into_owned_opt() {
            LayoutIterator(GeckoChildrenIterator::GeckoIterator(iter))
        } else {
            LayoutIterator(GeckoChildrenIterator::Current(self.first_child()))
        }
    }

    fn opaque(&self) -> OpaqueNode {
        let ptr: uintptr_t = self.0 as *const _ as uintptr_t;
        OpaqueNode(ptr)
    }

    fn layout_parent_node(self, reflow_root: OpaqueNode) -> Option<GeckoNode<'ln>> {
        if self.opaque() == reflow_root {
            None
        } else {
            self.parent_node()
        }
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

    fn as_document(&self) -> Option<GeckoDocument<'ln>> {
        unimplemented!()
    }

    // NOTE: This is not relevant for Gecko, since we get explicit restyle hints
    // when a content has changed.
    fn has_changed(&self) -> bool { false }

    unsafe fn set_changed(&self, _value: bool) {
        unimplemented!()
    }

    fn is_dirty(&self) -> bool {
        // Return true unconditionally if we're not yet styled. This is a hack
        // and should go away soon.
        if self.get_node_data().is_none() {
            return true;
        }

        self.flags() & (NODE_IS_DIRTY_FOR_SERVO as u32) != 0
    }

    unsafe fn set_dirty(&self, value: bool) {
        if value {
            self.set_flags(NODE_IS_DIRTY_FOR_SERVO as u32)
        } else {
            self.unset_flags(NODE_IS_DIRTY_FOR_SERVO as u32)
        }
    }

    fn has_dirty_descendants(&self) -> bool {
        // Return true unconditionally if we're not yet styled. This is a hack
        // and should go away soon.
        if self.get_node_data().is_none() {
            return true;
        }
        self.flags() & (NODE_HAS_DIRTY_DESCENDANTS_FOR_SERVO as u32) != 0
    }

    unsafe fn set_dirty_descendants(&self, value: bool) {
        if value {
            self.set_flags(NODE_HAS_DIRTY_DESCENDANTS_FOR_SERVO as u32)
        } else {
            self.unset_flags(NODE_HAS_DIRTY_DESCENDANTS_FOR_SERVO as u32)
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

    fn store_children_to_process(&self, _: isize) {
        // This is only used for bottom-up traversal, and is thus a no-op for Gecko.
    }

    fn did_process_child(&self) -> isize {
        panic!("Atomic child count not implemented in Gecko");
    }

    #[inline(always)]
    fn borrow_data(&self) -> Option<AtomicRef<PersistentStyleData>> {
        self.get_node_data().as_ref().map(|d| d.0.borrow())
    }

    #[inline(always)]
    fn mutate_data(&self) -> Option<AtomicRefMut<PersistentStyleData>> {
        self.get_node_data().as_ref().map(|d| d.0.borrow_mut())
    }

    fn restyle_damage(self) -> Self::ConcreteRestyleDamage {
        // Not called from style, only for layout.
        unimplemented!();
    }

    fn set_restyle_damage(self, damage: Self::ConcreteRestyleDamage) {
        unsafe { Gecko_StoreStyleDifference(self.0, damage.0) }
    }

    fn parent_node(&self) -> Option<GeckoNode<'ln>> {
        unsafe { self.0.mParent.as_ref().map(GeckoNode) }
    }

    fn first_child(&self) -> Option<GeckoNode<'ln>> {
        unsafe { self.0.mFirstChild.as_ref().map(GeckoNode::from_content) }
    }

    fn last_child(&self) -> Option<GeckoNode<'ln>> {
        unsafe { Gecko_GetLastChild(self.0).map(GeckoNode) }
    }

    fn prev_sibling(&self) -> Option<GeckoNode<'ln>> {
        unsafe { self.0.mPreviousSibling.as_ref().map(GeckoNode::from_content) }
    }

    fn next_sibling(&self) -> Option<GeckoNode<'ln>> {
        unsafe { self.0.mNextSibling.as_ref().map(GeckoNode::from_content) }
    }

    fn existing_style_for_restyle_damage<'a>(&'a self,
                                             current_cv: Option<&'a Arc<ComputedValues>>,
                                             pseudo: Option<&PseudoElement>)
                                             -> Option<&'a nsStyleContext> {
        if current_cv.is_none() {
            // Don't bother in doing an ffi call to get null back.
            return None;
        }

        unsafe {
            let atom_ptr = pseudo.map(|p| p.as_atom().as_ptr())
                                 .unwrap_or(ptr::null_mut());
            let context_ptr = Gecko_GetStyleContext(self.0, atom_ptr);
            context_ptr.as_ref()
        }
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

// We generally iterate children by traversing the siblings of the first child
// like Servo does. However, for nodes with anonymous children, we use a custom
// (heavier-weight) Gecko-implemented iterator.
pub enum GeckoChildrenIterator<'a> {
    Current(Option<GeckoNode<'a>>),
    GeckoIterator(bindings::StyleChildrenIteratorOwned),
}

impl<'a> Drop for GeckoChildrenIterator<'a> {
    fn drop(&mut self) {
        if let GeckoChildrenIterator::GeckoIterator(ref it) = *self {
            unsafe {
                Gecko_DropStyleChildrenIterator(ptr::read(it as *const _));
            }
        }
    }
}
