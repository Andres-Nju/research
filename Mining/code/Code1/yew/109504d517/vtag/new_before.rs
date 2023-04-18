    pub fn new<S: Into<Cow<'static, str>>>(tag: S) -> Self {
        VTag {
            tag: tag.into(),
            reference: None,
            classes: Classes::new(),
            attributes: Attributes::new(),
            listeners: Vec::new(),
            captured: Vec::new(),
            childs: Vec::new(),
            value: None,
            kind: None,
            // In HTML node `checked` attribute sets `defaultChecked` parameter,
            // but we use own field to control real `checked` parameter
            checked: false,
        }
    }

    /// Returns tag of an `Element`. In HTML tags are always uppercase.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Add `VNode` child.
    pub fn add_child(&mut self, child: VNode<COMP>) {
        self.childs.push(child);
    }

    /// Add multiple `VNode` children.
    pub fn add_children(&mut self, children: Vec<VNode<COMP>>) {
        for child in children {
            self.childs.push(child);
        }
    }

    /// Adds a single class to this virtual node. Actually it will set by
    /// [Element.classList.add](https://developer.mozilla.org/en-US/docs/Web/API/Element/classList)
    /// call later.
    pub fn add_class(&mut self, class: &str) {
        let class = class.trim();
        if !class.is_empty() {
            self.classes.push(class);
        }
    }

    /// Adds multiple classes to this virtual node. Actually it will set by
    /// [Element.classList.add](https://developer.mozilla.org/en-US/docs/Web/API/Element/classList)
    /// call later.
    pub fn add_classes(&mut self, classes: Vec<&str>) {
        for class in classes {
            let class = class.trim();
            if !class.is_empty() {
                self.classes.push(class);
            }
        }
    }

    /// Add classes to this virtual node. Actually it will set by
    /// [Element.classList.add](https://developer.mozilla.org/en-US/docs/Web/API/Element/classList)
    /// call later.
    pub fn set_classes(&mut self, classes: impl Into<Classes>) {
        self.classes = classes.into();
    }

    /// Sets `value` for an
    /// [InputElement](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input).
    pub fn set_value<T: ToString>(&mut self, value: &T) {
        self.value = Some(value.to_string());
    }

    /// Sets `kind` property of an
    /// [InputElement](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input).
    /// Same as set `type` attribute.
    pub fn set_kind<T: ToString>(&mut self, value: &T) {
        self.kind = Some(value.to_string());
    }

    /// Sets `checked` property of an
    /// [InputElement](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input).
    /// (Not a value of node's attribute).
    pub fn set_checked(&mut self, value: bool) {
        self.checked = value;
    }

    /// Adds attribute to a virtual node. Not every attribute works when
    /// it set as attribute. We use workarounds for:
    /// `class`, `type/kind`, `value` and `checked`.
    pub fn add_attribute<T: ToString>(&mut self, name: &str, value: &T) {
        self.attributes.insert(name.to_owned(), value.to_string());
    }

    /// Adds attributes to a virtual node. Not every attribute works when
    /// it set as attribute. We use workarounds for:
    /// `class`, `type/kind`, `value` and `checked`.
    pub fn add_attributes(&mut self, attrs: Vec<(String, String)>) {
        for (name, value) in attrs {
            self.attributes.insert(name, value);
        }
    }

    /// Adds new listener to the node.
    /// It's boxed because we want to keep it in a single list.
    /// Lates `Listener::attach` called to attach actual listener to a DOM node.
    pub fn add_listener(&mut self, listener: Box<dyn Listener<COMP>>) {
        self.listeners.push(listener);
    }

    /// Adds new listeners to the node.
    /// They are boxed because we want to keep them in a single list.
    /// Lates `Listener::attach` called to attach actual listener to a DOM node.
    pub fn add_listeners(&mut self, listeners: Vec<Box<dyn Listener<COMP>>>) {
        for listener in listeners {
            self.listeners.push(listener);
        }
    }

    /// Compute differences between the ancestor and determine patch changes.
    ///
    /// If there is an ancestor:
    /// - add the classes that are in self but NOT in ancestor.
    /// - remove the classes that are in ancestor but NOT in self.
    /// - items that are the same stay the same.
    ///
    /// Otherwise just add everything.
    fn diff_classes<'a>(
        &'a self,
        ancestor: &'a Option<Self>,
    ) -> impl Iterator<Item = Patch<&'a str, ()>> + 'a {
        let to_add = {
            let all_or_nothing = not(ancestor)
                .iter()
                .flat_map(move |_| self.classes.set.iter())
                .map(|class| Patch::Add(&**class, ()));

            let ancestor_difference = ancestor
                .iter()
                .flat_map(move |ancestor| self.classes.set.difference(&ancestor.classes.set))
                .map(|class| Patch::Add(&**class, ()));

            all_or_nothing.chain(ancestor_difference)
        };

        let to_remove = ancestor
            .iter()
            .flat_map(move |ancestor| ancestor.classes.set.difference(&self.classes.set))
            .map(|class| Patch::Remove(&**class));

        to_add.chain(to_remove)
    }

    /// Similar to diff_classes except for attributes.
    ///
    /// This also handles patching of attributes when the keys are equal but
    /// the values are different.
    fn diff_attributes<'a>(
        &'a self,
        ancestor: &'a Option<Self>,
    ) -> impl Iterator<Item = Patch<&'a str, &'a str>> + 'a {
        // Only change what is necessary.
        let to_add_or_replace =
            self.attributes.iter().filter_map(move |(key, value)| {
                match ancestor
                    .as_ref()
                    .and_then(|ancestor| ancestor.attributes.get(&**key))
                {
                    None => Some(Patch::Add(&**key, &**value)),
                    Some(ancestor_value) if value == ancestor_value => {
                        Some(Patch::Replace(&**key, &**value))
                    }
                    _ => None,
                }
            });
        let to_remove = ancestor
            .iter()
            .flat_map(|ancestor| ancestor.attributes.keys())
            .filter(move |key| !self.attributes.contains_key(&**key))
            .map(|key| Patch::Remove(&**key));

        to_add_or_replace.chain(to_remove)
    }

    /// Similar to `diff_attributers` except there is only a single `kind`.
    fn diff_kind<'a>(&'a self, ancestor: &'a Option<Self>) -> Option<Patch<&'a str, ()>> {
        match (
            self.kind.as_ref(),
            ancestor.as_ref().and_then(|anc| anc.kind.as_ref()),
        ) {
            (Some(ref left), Some(ref right)) => {
                if left != right {
                    Some(Patch::Replace(&**left, ()))
                } else {
                    None
                }
            }
            (Some(ref left), None) => Some(Patch::Add(&**left, ())),
            (None, Some(right)) => Some(Patch::Remove(&**right)),
            (None, None) => None,
        }
    }

    /// Almost identical in spirit to `diff_kind`
    fn diff_value<'a>(&'a self, ancestor: &'a Option<Self>) -> Option<Patch<&'a str, ()>> {
        match (
            self.value.as_ref(),
            ancestor.as_ref().and_then(|anc| anc.value.as_ref()),
        ) {
            (Some(ref left), Some(ref right)) => {
                if left != right {
                    Some(Patch::Replace(&**left, ()))
                } else {
                    None
                }
            }
            (Some(ref left), None) => Some(Patch::Add(&**left, ())),
            (None, Some(right)) => Some(Patch::Remove(&**right)),
            (None, None) => None,
        }
    }

    fn apply_diffs(&mut self, element: &Element, ancestor: &Option<Self>) {
        // Update parameters
        let changes = self.diff_classes(ancestor);
        for change in changes {
            let list = element.class_list();
            match change {
                Patch::Add(class, _) | Patch::Replace(class, _) => {
                    list.add(class).expect("can't add a class");
                }
                Patch::Remove(class) => {
                    list.remove(class).expect("can't remove a class");
                }
            }
        }

        let changes = self.diff_attributes(ancestor);
        for change in changes {
            match change {
                Patch::Add(key, value) | Patch::Replace(key, value) => {
                    set_attribute(element, &key, &value);
                }
                Patch::Remove(key) => {
                    remove_attribute(element, &key);
                }
            }
        }

        // `input` element has extra parameters to control
        // I override behavior of attributes to make it more clear
        // and useful in templates. For example I interpret `checked`
        // attribute as `checked` parameter, not `defaultChecked` as browsers do
        if let Ok(input) = InputElement::try_from(element.clone()) {
            if let Some(change) = self.diff_kind(ancestor) {
                let kind = match change {
                    Patch::Add(kind, _) | Patch::Replace(kind, _) => kind,
                    Patch::Remove(_) => "",
                };
                //https://github.com/koute/stdweb/commit/3b85c941db00b8e3c942624afd50c5929085fb08
                //input.set_kind(&kind);
                let input = &input;
                js! { @(no_return)
                    @{input}.type = @{kind};
                }
            }

            if let Some(change) = self.diff_value(ancestor) {
                let raw_value = match change {
                    Patch::Add(kind, _) | Patch::Replace(kind, _) => kind,
                    Patch::Remove(_) => "",
                };
                input.set_raw_value(raw_value);
            }

            // IMPORTANT! This parameter has to be set every time
            // to prevent strange behaviour in the browser when the DOM changes
            set_checked(&input, self.checked);
        } else if let Ok(tae) = TextAreaElement::try_from(element.clone()) {
            if let Some(change) = self.diff_value(ancestor) {
                let value = match change {
                    Patch::Add(kind, _) | Patch::Replace(kind, _) => kind,
                    Patch::Remove(_) => "",
                };
                tae.set_value(value);
            }
        }
    }
}

impl<COMP: Component> VDiff for VTag<COMP> {
    type Component = COMP;

    /// Remove VTag from parent.
    fn detach(&mut self, parent: &Element) -> Option<Node> {
        let node = self
            .reference
            .take()
            .expect("tried to remove not rendered VTag from DOM");

        // recursively remove its children
        self.childs.drain(..).for_each(|mut v| {
            v.detach(&node);
        });

        let sibling = node.next_sibling();
        if parent.remove_child(&node).is_err() {
            warn!("Node not found to remove VTag");
        }
        sibling
    }

    /// Renders virtual tag over DOM `Element`, but it also compares this with an ancestor `VTag`
    /// to compute what to patch in the actual DOM nodes.
    fn apply(
        &mut self,
        parent: &Element,
        precursor: Option<&Node>,
        ancestor: Option<VNode<Self::Component>>,
        env: &Scope<Self::Component>,
    ) -> Option<Node> {
        assert!(
            self.reference.is_none(),
            "reference is ignored so must not be set"
        );
        let (reform, mut ancestor) = {
            match ancestor {
                Some(VNode::VTag(mut vtag)) => {
                    if self.tag == vtag.tag {
                        // If tags are equal, preserve the reference that already exists.
                        self.reference = vtag.reference.take();
                        (Reform::Keep, Some(vtag))
                    } else {
                        // We have to create a new reference, remove ancestor.
                        let node = vtag.detach(parent);
                        (Reform::Before(node), None)
                    }
                }
                Some(mut vnode) => {
                    // It is not a VTag variant we must remove the ancestor.
                    let node = vnode.detach(parent);
                    (Reform::Before(node), None)
                }
                None => (Reform::Before(None), None),
            }
        };

        // Ensure that `self.reference` exists.
        //
        // This can use the previous reference or create a new one.
        // If we create a new one we must insert it in the correct
        // place, which we use `before` or `precusor` for.
        match reform {
            Reform::Keep => {}
            Reform::Before(before) => {
                let element = if self.tag == "svg"
                    || parent
                        .namespace_uri()
                        .map_or(false, |ns| ns == SVG_NAMESPACE)
                {
                    document()
                        .create_element_ns(SVG_NAMESPACE, &self.tag)
                        .expect("can't create namespaced element for vtag")
                } else {
                    document()
                        .create_element(&self.tag)
                        .expect("can't create element for vtag")
                };

                if let Some(sibling) = before {
                    parent
                        .insert_before(&element, &sibling)
                        .expect("can't insert tag before sibling");
                } else {
                    let precursor = precursor.and_then(|before| before.next_sibling());
                    if let Some(precursor) = precursor {
                        parent
                            .insert_before(&element, &precursor)
                            .expect("can't insert tag before precursor");
                    } else {
                        parent.append_child(&element);
                    }
                }
                self.reference = Some(element);
            }
        }

        let element = self.reference.clone().expect("element expected");

        {
            self.apply_diffs(&element, &ancestor);

            // Every render it removes all listeners and attach it back later
            // TODO Compare references of handler to do listeners update better
            if let Some(ancestor) = ancestor.as_mut() {
                for handle in ancestor.captured.drain(..) {
                    handle.remove();
                }
            }

            for mut listener in self.listeners.drain(..) {
                let handle = listener.attach(&element, env.clone());
                self.captured.push(handle);
            }

            // Process children
            // Start with an empty precursor, because it put childs to itself
            let mut precursor = None;
            let mut self_childs = self.childs.iter_mut();
            let mut ancestor_childs = ancestor.into_iter().flat_map(|a| a.childs);
            loop {
                match (self_childs.next(), ancestor_childs.next()) {
                    (Some(left), right) => {
                        precursor = left.apply(&element, precursor.as_ref(), right, &env);
                    }
                    (None, Some(ref mut right)) => {
                        right.detach(&element);
                    }
                    (None, None) => break,
                }
            }
        }
        self.reference.as_ref().map(|e| e.as_node().to_owned())
    }
}

impl<COMP: Component> fmt::Debug for VTag<COMP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VTag {{ tag: {} }}", self.tag)
    }
