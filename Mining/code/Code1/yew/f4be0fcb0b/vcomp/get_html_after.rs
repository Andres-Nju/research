    fn get_html(mut node: Html, scope: &AnyScope, parent: &Element) -> String {
        use super::VDiff;

        // clear parent
        parent.set_inner_html("");

        node.apply(&scope, &parent, NodeRef::default(), None);
        parent.inner_html()
    }
