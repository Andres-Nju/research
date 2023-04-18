        fn visit_node<T>(node: &T, label: &str) -> Option<String>
        where
            T: NameOwner + VisibilityOwner,
        {
            let mut string =
                node.visibility().map(|v| format!("{} ", v.syntax().text())).unwrap_or_default();
            string.push_str(label);
            string.push_str(node.name()?.text().as_str());
            Some(string)
        }
