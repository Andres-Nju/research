    pub fn invalidate(mut self) -> InvalidationResult {
        debug!("StyleTreeInvalidator::invalidate({:?})", self.element);
        debug_assert!(self.element.has_snapshot(), "Why bothering?");
        debug_assert!(self.data.is_some(), "How exactly?");

        let shared_context = self.shared_context;

        let wrapper =
            ElementWrapper::new(self.element, shared_context.snapshot_map);
        let state_changes = wrapper.state_changes();
        let snapshot = wrapper.snapshot().expect("has_snapshot lied");

        if !snapshot.has_attrs() && state_changes.is_empty() {
            return InvalidationResult::empty();
        }

        // If we are sensitive to visitedness and the visited state changed, we
        // force a restyle here. Matching doesn't depend on the actual visited
        // state at all, so we can't look at matching results to decide what to
        // do for this case.
        if state_changes.intersects(IN_VISITED_OR_UNVISITED_STATE) {
            trace!(" > visitedness change, force subtree restyle");
            // We can't just return here because there may also be attribute
            // changes as well that imply additional hints.
            let data = self.data.as_mut().unwrap();
            data.hint.insert(RestyleHint::restyle_subtree());
        }

        let mut classes_removed = SmallVec::<[Atom; 8]>::new();
        let mut classes_added = SmallVec::<[Atom; 8]>::new();
        if snapshot.class_changed() {
            // TODO(emilio): Do this more efficiently!
            snapshot.each_class(|c| {
                if !self.element.has_class(c, CaseSensitivity::CaseSensitive) {
                    classes_removed.push(c.clone())
                }
            });

            self.element.each_class(|c| {
                if !snapshot.has_class(c, CaseSensitivity::CaseSensitive) {
                    classes_added.push(c.clone())
                }
            })
        }

        let mut id_removed = None;
        let mut id_added = None;
        if snapshot.id_changed() {
            let old_id = snapshot.id_attr();
            let current_id = self.element.get_id();

            if old_id != current_id {
                id_removed = old_id;
                id_added = current_id;
            }
        }

        let lookup_element =
            if self.element.implemented_pseudo_element().is_some() {
                self.element.pseudo_element_originating_element().unwrap()
            } else {
                self.element
            };

        let mut descendant_invalidations = InvalidationVector::new();
        let mut sibling_invalidations = InvalidationVector::new();
        let invalidated_self = {
            let mut collector = InvalidationCollector {
                wrapper,
                lookup_element,
                nth_index_cache: self.nth_index_cache.as_mut().map(|c| &mut **c),
                state_changes,
                element: self.element,
                snapshot: &snapshot,
                quirks_mode: self.shared_context.quirks_mode(),
                removed_id: id_removed.as_ref(),
                added_id: id_added.as_ref(),
                classes_removed: &classes_removed,
                classes_added: &classes_added,
                descendant_invalidations: &mut descendant_invalidations,
                sibling_invalidations: &mut sibling_invalidations,
                invalidates_self: false,
            };

            shared_context.stylist.each_invalidation_map(|invalidation_map| {
                collector.collect_dependencies_in_invalidation_map(invalidation_map);
            });

            // TODO(emilio): Consider storing dependencies from the UA sheet in
            // a different map. If we do that, we can skip the stuff on the
            // shared stylist iff cut_off_inheritance is true, and we can look
            // just at that map.
            let _cut_off_inheritance =
                self.element.each_xbl_stylist(|stylist| {
                    stylist.each_invalidation_map(|invalidation_map| {
                        collector.collect_dependencies_in_invalidation_map(invalidation_map);
                    });
                });

            collector.invalidates_self
        };

        if invalidated_self {
            if let Some(ref mut data) = self.data {
                data.hint.insert(RESTYLE_SELF);
            }
        }

        debug!("Collected invalidations (self: {}): ", invalidated_self);
        debug!(" > descendants: {:?}", descendant_invalidations);
        debug!(" > siblings: {:?}", sibling_invalidations);
        let invalidated_descendants = self.invalidate_descendants(&descendant_invalidations);
        let invalidated_siblings = self.invalidate_siblings(&mut sibling_invalidations);

        InvalidationResult { invalidated_self, invalidated_descendants, invalidated_siblings }
    }
