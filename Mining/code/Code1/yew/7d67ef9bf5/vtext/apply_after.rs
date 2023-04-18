    fn apply(
        &mut self,
        parent: &Element,
        previous_sibling: Option<&Node>,
        ancestor: Option<VNode>,
    ) -> Option<Node> {
        assert!(
            self.reference.is_none(),
            "reference is ignored so must not be set"
        );
        let reform = {
            match ancestor {
                // If element matched this type
                Some(VNode::VText(mut vtext)) => {
                    self.reference = vtext.reference.take();
                    if self.text != vtext.text {
                        if let Some(ref element) = self.reference {
                            element.set_node_value(Some(&self.text));
                        }
                    }
                    Reform::Keep
                }
                Some(mut vnode) => Reform::Before(vnode.detach(parent)),
                None => Reform::Before(None),
            }
        };
        match reform {
            Reform::Keep => {}
            Reform::Before(next_sibling) => {
                let element = document().create_text_node(&self.text);
                if let Some(next_sibling) = next_sibling {
                    let next_sibling = &next_sibling;
                    #[cfg(feature = "web_sys")]
                    let next_sibling = Some(next_sibling);
                    parent
                        .insert_before(&element, next_sibling)
                        .expect("can't insert text before the next sibling");
                } else if let Some(next_sibling) = previous_sibling.and_then(|p| p.next_sibling()) {
                    let next_sibling = &next_sibling;
                    #[cfg(feature = "web_sys")]
                    let next_sibling = Some(next_sibling);
                    parent
                        .insert_before(&element, next_sibling)
                        .expect("can't insert text before next_sibling");
                } else {
                    #[cfg_attr(
                        feature = "std_web",
                        allow(clippy::let_unit_value, unused_variables)
                    )]
                    {
                        let result = parent.append_child(&element);
                        #[cfg(feature = "web_sys")]
                        result.expect("can't append node to parent");
                    }
                }
                self.reference = Some(element);
            }
        }
        self.reference.as_ref().map(|t| {
            let node = cfg_match! {
                feature = "std_web" => t.as_node(),
                feature = "web_sys" => t.deref().deref(),
            };
            node.to_owned()
        })
    }
