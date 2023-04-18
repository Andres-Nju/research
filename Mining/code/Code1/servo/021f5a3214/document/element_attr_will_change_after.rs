    pub fn element_attr_will_change(&self, el: &Element, attr: &Attr) {
        // FIXME(emilio): Kind of a shame we have to duplicate this.
        //
        // I'm getting rid of the whole hashtable soon anyway, since all it does
        // right now is populate the element restyle data in layout, and we
        // could in theory do it in the DOM I think.
        let mut entry = self.ensure_pending_restyle(el);
        if entry.snapshot.is_none() {
            entry.snapshot = Some(Snapshot::new(el.html_element_in_html_document()));
        }
        if attr.local_name() == &local_name!("style") {
            entry.hint |= RESTYLE_STYLE_ATTRIBUTE;
        }

        // FIXME(emilio): This should become something like
        // element.is_attribute_mapped(attr.local_name()).
        if attr.local_name() == &local_name!("width") ||
           attr.local_name() == &local_name!("height") {
            entry.hint |= RESTYLE_SELF;
        }

        let mut snapshot = entry.snapshot.as_mut().unwrap();
        if snapshot.attrs.is_none() {
            let attrs = el.attrs()
                          .iter()
                          .map(|attr| (attr.identifier().clone(), attr.value().clone()))
                          .collect();
            snapshot.attrs = Some(attrs);
        }
    }
