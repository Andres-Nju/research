    fn snapshot(&self) -> Option<&'a Snapshot> {
        if !self.element.has_snapshot() {
            return None;
        }

        if let Some(s) = self.cached_snapshot.get() {
            return Some(s);
        }

        let snapshot = self.snapshot_map.get(&self.element);
        debug_assert!(snapshot.is_some(), "has_snapshot lied!");

        self.cached_snapshot.set(snapshot);

        snapshot
    }
}

impl<'a, E> MatchAttr for ElementWrapper<'a, E>
    where E: TElement,
{
    type Impl = SelectorImpl;

    fn match_attr_has(&self, attr: &AttrSelector<SelectorImpl>) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.match_attr_has(attr),
            _   => self.element.match_attr_has(attr)
        }
    }

    fn match_attr_equals(&self,
                         attr: &AttrSelector<SelectorImpl>,
                         value: &AttrValue) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.match_attr_equals(attr, value),
            _   => self.element.match_attr_equals(attr, value)
        }
    }

    fn match_attr_equals_ignore_ascii_case(&self,
                                           attr: &AttrSelector<SelectorImpl>,
                                           value: &AttrValue) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.match_attr_equals_ignore_ascii_case(attr, value),
            _   => self.element.match_attr_equals_ignore_ascii_case(attr, value)
        }
    }

    fn match_attr_includes(&self,
                           attr: &AttrSelector<SelectorImpl>,
                           value: &AttrValue) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.match_attr_includes(attr, value),
            _   => self.element.match_attr_includes(attr, value)
        }
    }

    fn match_attr_dash(&self,
                       attr: &AttrSelector<SelectorImpl>,
                       value: &AttrValue) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.match_attr_dash(attr, value),
            _   => self.element.match_attr_dash(attr, value)
        }
    }

    fn match_attr_prefix(&self,
                         attr: &AttrSelector<SelectorImpl>,
                         value: &AttrValue) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.match_attr_prefix(attr, value),
            _   => self.element.match_attr_prefix(attr, value)
        }
    }

    fn match_attr_substring(&self,
                            attr: &AttrSelector<SelectorImpl>,
                            value: &AttrValue) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.match_attr_substring(attr, value),
            _   => self.element.match_attr_substring(attr, value)
        }
    }

    fn match_attr_suffix(&self,
                         attr: &AttrSelector<SelectorImpl>,
                         value: &AttrValue) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.match_attr_suffix(attr, value),
            _   => self.element.match_attr_suffix(attr, value)
        }
    }
}

impl<'a, E> Element for ElementWrapper<'a, E>
    where E: TElement,
{
    fn match_non_ts_pseudo_class<F>(&self,
                                    pseudo_class: &NonTSPseudoClass,
                                    relations: &mut StyleRelations,
                                    _setter: &mut F)
                                    -> bool
        where F: FnMut(&Self, ElementSelectorFlags),
    {
        // :moz-any is quite special, because we need to keep matching as a
        // snapshot.
        #[cfg(feature = "gecko")]
        {
            use selectors::matching::matches_complex_selector;
            if let NonTSPseudoClass::MozAny(ref selectors) = *pseudo_class {
                return selectors.iter().any(|s| {
                    matches_complex_selector(s, self, relations, _setter)
                })
            }
        }

        let flag = pseudo_class.state_flag();
        if flag.is_empty() {
            return self.element.match_non_ts_pseudo_class(pseudo_class,
                                                          relations,
                                                          &mut |_, _| {})
        }
        match self.snapshot().and_then(|s| s.state()) {
            Some(snapshot_state) => snapshot_state.intersects(flag),
            None => {
                self.element.match_non_ts_pseudo_class(pseudo_class,
                                                       relations,
                                                       &mut |_, _| {})
            }
        }
    }

    fn parent_element(&self) -> Option<Self> {
        self.element.parent_element()
            .map(|e| ElementWrapper::new(e, self.snapshot_map))
    }

    fn first_child_element(&self) -> Option<Self> {
        self.element.first_child_element()
            .map(|e| ElementWrapper::new(e, self.snapshot_map))
    }

    fn last_child_element(&self) -> Option<Self> {
        self.element.last_child_element()
            .map(|e| ElementWrapper::new(e, self.snapshot_map))
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.element.prev_sibling_element()
            .map(|e| ElementWrapper::new(e, self.snapshot_map))
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.element.next_sibling_element()
            .map(|e| ElementWrapper::new(e, self.snapshot_map))
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.element.is_html_element_in_html_document()
    }

    fn get_local_name(&self) -> &<Self::Impl as ::selectors::SelectorImpl>::BorrowedLocalName {
        self.element.get_local_name()
    }

    fn get_namespace(&self) -> &<Self::Impl as ::selectors::SelectorImpl>::BorrowedNamespaceUrl {
        self.element.get_namespace()
    }

    fn get_id(&self) -> Option<Atom> {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.id_attr(),
            _   => self.element.get_id()
        }
    }

    fn has_class(&self, name: &Atom) -> bool {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.has_class(name),
            _   => self.element.has_class(name)
        }
    }

    fn is_empty(&self) -> bool {
        self.element.is_empty()
    }

    fn is_root(&self) -> bool {
        self.element.is_root()
    }

    fn each_class<F>(&self, callback: F)
        where F: FnMut(&Atom) {
        match self.snapshot() {
            Some(snapshot) if snapshot.has_attrs()
                => snapshot.each_class(callback),
            _   => self.element.each_class(callback)
        }
    }
}

fn selector_to_state(sel: &Component<SelectorImpl>) -> ElementState {
    match *sel {
        Component::NonTSPseudoClass(ref pc) => pc.state_flag(),
        _ => ElementState::empty(),
    }
}

fn is_attr_selector(sel: &Component<SelectorImpl>) -> bool {
    match *sel {
        Component::ID(_) |
        Component::Class(_) |
        Component::AttrExists(_) |
        Component::AttrEqual(_, _, _) |
        Component::AttrIncludes(_, _) |
        Component::AttrDashMatch(_, _) |
        Component::AttrPrefixMatch(_, _) |
        Component::AttrSubstringMatch(_, _) |
        Component::AttrSuffixMatch(_, _) => true,
        _ => false,
    }
}

fn combinator_to_restyle_hint(combinator: Option<Combinator>) -> RestyleHint {
    match combinator {
        None => RESTYLE_SELF,
        Some(c) => match c {
            Combinator::Child => RESTYLE_DESCENDANTS,
            Combinator::Descendant => RESTYLE_DESCENDANTS,
            Combinator::NextSibling => RESTYLE_LATER_SIBLINGS,
            Combinator::LaterSibling => RESTYLE_LATER_SIBLINGS,
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
/// The aspects of an selector which are sensitive.
pub struct Sensitivities {
    /// The states which are sensitive.
    pub states: ElementState,
    /// Whether attributes are sensitive.
    pub attrs: bool,
}

impl Sensitivities {
    fn is_empty(&self) -> bool {
        self.states.is_empty() && !self.attrs
    }

    fn new() -> Sensitivities {
        Sensitivities {
            states: ElementState::empty(),
            attrs: false,
        }
    }

    fn sensitive_to(&self, attrs: bool, states: ElementState) -> bool {
        (attrs && self.attrs) || self.states.intersects(states)
    }
}

/// Mapping between (partial) CompoundSelectors (and the combinator to their
/// right) and the states and attributes they depend on.
///
/// In general, for all selectors in all applicable stylesheets of the form:
///
/// |a _ b _ c _ d _ e|
///
/// Where:
///   * |b| and |d| are simple selectors that depend on state (like :hover) or
///     attributes (like [attr...], .foo, or #foo).
///   * |a|, |c|, and |e| are arbitrary simple selectors that do not depend on
///     state or attributes.
///
/// We generate a Dependency for both |a _ b:X _| and |a _ b:X _ c _ d:Y _|,
/// even though those selectors may not appear on their own in any stylesheet.
/// This allows us to quickly scan through the dependency sites of all style
/// rules and determine the maximum effect that a given state or attribute
/// change may have on the style of elements in the document.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub struct Dependency {
    #[cfg_attr(feature = "servo", ignore_heap_size_of = "Arc")]
    selector: SelectorInner<SelectorImpl>,
    /// The hint associated with this dependency.
    pub hint: RestyleHint,
    /// The sensitivities associated with this dependency.
    pub sensitivities: Sensitivities,
}

impl Borrow<SelectorInner<SelectorImpl>> for Dependency {
    fn borrow(&self) -> &SelectorInner<SelectorImpl> {
        &self.selector
    }
}

/// The following visitor visits all the simple selectors for a given complex
/// selector, taking care of :not and :any combinators, collecting whether any
/// of them is sensitive to attribute or state changes.
struct SensitivitiesVisitor {
    sensitivities: Sensitivities,
}

impl SelectorVisitor for SensitivitiesVisitor {
    type Impl = SelectorImpl;
    fn visit_simple_selector(&mut self, s: &Component<SelectorImpl>) -> bool {
        self.sensitivities.states.insert(selector_to_state(s));
        self.sensitivities.attrs |= is_attr_selector(s);
        true
    }
}

/// A set of dependencies for a given stylist.
///
/// Note that we can have many dependencies, often more than the total number
/// of selectors given that we can get multiple partial selectors for a given
/// selector. As such, we want all the usual optimizations, including the
/// SelectorMap and the bloom filter.
#[derive(Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub struct DependencySet(pub SelectorMap<Dependency>);

impl DependencySet {
    fn add_dependency(&mut self, dep: Dependency) {
        self.0.insert(dep);
    }

    /// Adds a selector to this `DependencySet`.
    pub fn note_selector(&mut self, selector: &Selector<SelectorImpl>) {
        let mut combinator = None;
        let mut iter = selector.inner.complex.iter();
        let mut index = 0;

        loop {
            let sequence_start = index;
            let mut visitor = SensitivitiesVisitor {
                sensitivities: Sensitivities::new()
            };

            // Visit all the simple selectors in this sequence.
            //
            // Note that this works because we can't have combinators nested
            // inside simple selectors (i.e. in :not() or :-moz-any()). If we
            // ever support that we'll need to visit complex selectors as well.
            for ss in &mut iter {
                ss.visit(&mut visitor);
                index += 1; // Account for the simple selector.
            }
