    fn update_animations(&self,
                         pseudo: Option<&PseudoElement>,
                         before_change_style: Option<Arc<ComputedValues>>,
                         tasks: UpdateAnimationsTasks) {
        // We have to update animations even if the element has no computed style
        // since it means the element is in a display:none subtree, we should destroy
        // all CSS animations in display:none subtree.
        let computed_data = self.borrow_data();
        let computed_values =
            computed_data.as_ref().map(|d|
                pseudo.map_or_else(|| d.styles().primary.values(),
                                   |p| d.styles().pseudos.get(p).unwrap().values())
            );
        let computed_values_opt = computed_values.map(|v|
            *HasArcFFI::arc_as_borrowed(v)
        );

        let parent_element = if pseudo.is_some() {
            self.parent_element()
        } else {
            Some(*self)
        };
        let parent_data = parent_element.as_ref().and_then(|e| e.borrow_data());
        let parent_values = parent_data.as_ref().map(|d| d.styles().primary.values());
        let parent_values_opt = parent_values.map(|v|
            *HasArcFFI::arc_as_borrowed(v)
        );

        let atom_ptr = PseudoElement::ns_atom_or_null_from_opt(pseudo);
        let before_change_values = before_change_style.as_ref().map(|v| *HasArcFFI::arc_as_borrowed(v));
        unsafe {
            Gecko_UpdateAnimations(self.0, atom_ptr,
                                   before_change_values,
                                   computed_values_opt,
                                   parent_values_opt,
                                   tasks.bits());
        }
    }
