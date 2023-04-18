    fn get_html(mut node: Html, scope: &AnyScope, parent: &Element) -> String {
        use super::VDiff;

        // clear parent
        parent.set_inner_html("");

        node.apply(&scope, &parent, None, None);
        parent.inner_html()
    }
