    pub fn lazily_compute_pseudo_element_style<E>(&self,
                                                  guards: &StylesheetGuards,
                                                  element: &E,
                                                  pseudo: &PseudoElement,
                                                  parent: &Arc<ComputedValues>,
                                                  font_metrics: &FontMetricsProvider)
                                                  -> Option<ComputedStyle>
        where E: TElement +
                 fmt::Debug +
                 PresentationalHintsSynthetizer
    {
        debug_assert!(pseudo.is_lazy());
        if self.pseudos_map.get(pseudo).is_none() {
            return None;
        }

        let mut declarations = vec![];

        // Apply the selector flags. We should be in sequential mode
        // already, so we can directly apply the parent flags.
        let mut set_selector_flags = |element: &E, flags: ElementSelectorFlags| {
            if cfg!(feature = "servo") {
                // Servo calls this function from the worker, but only for internal
                // pseudos, so we should never generate selector flags here.
                unreachable!("internal pseudo generated slow selector flags?");
            }

            // Gecko calls this from sequential mode, so we can directly apply
            // the flags.
            debug_assert!(thread_state::get() == thread_state::LAYOUT);
            let self_flags = flags.for_self();
            if !self_flags.is_empty() {
                unsafe { element.set_selector_flags(self_flags); }
            }
            let parent_flags = flags.for_parent();
            if !parent_flags.is_empty() {
                if let Some(p) = element.parent_element() {
                    unsafe { p.set_selector_flags(parent_flags); }
                }
            }
        };


        self.push_applicable_declarations(element,
                                          None,
                                          None,
                                          None,
                                          AnimationRules(None, None),
                                          Some(pseudo),
                                          guards,
                                          &mut declarations,
                                          &mut set_selector_flags);

        let rule_node =
            self.rule_tree.insert_ordered_rules(
                declarations.into_iter().map(|a| (a.source, a.level)));

        // Read the comment on `precomputed_values_for_pseudo` to see why it's
        // difficult to assert that display: contents nodes never arrive here
        // (tl;dr: It doesn't apply for replaced elements and such, but the
        // computed value is still "contents").
        let computed =
            properties::cascade(&self.device,
                                &rule_node,
                                Some(pseudo),
                                guards,
                                Some(&**parent),
                                Some(&**parent),
                                None,
                                &RustLogReporter,
                                font_metrics,
                                CascadeFlags::empty());

        Some(ComputedStyle::new(rule_node, Arc::new(computed)))
    }
