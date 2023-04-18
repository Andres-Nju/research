    fn description(&self, db: &RootDatabase) -> Option<String> {
        // TODO: After type inference is done, add type information to improve the output
        let node = self.node(db)?;

        fn visit_ascribed_node<T>(node: &T, prefix: &str) -> Option<String>
        where
            T: NameOwner + VisibilityOwner + TypeAscriptionOwner,
        {
            let mut string = visit_node(node, prefix)?;

            if let Some(type_ref) = node.ascribed_type() {
                string.push_str(": ");
                type_ref.syntax().text().push_to(&mut string);
            }

            Some(string)
        }

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

        visitor()
            .visit(crate::completion::function_label)
            .visit(|node: &ast::StructDef| visit_node(node, "struct "))
            .visit(|node: &ast::EnumDef| visit_node(node, "enum "))
            .visit(|node: &ast::TraitDef| visit_node(node, "trait "))
            .visit(|node: &ast::Module| visit_node(node, "mod "))
            .visit(|node: &ast::TypeAliasDef| visit_node(node, "type "))
            .visit(|node: &ast::ConstDef| visit_ascribed_node(node, "const "))
            .visit(|node: &ast::StaticDef| visit_ascribed_node(node, "static "))
            .visit(|node: &ast::NamedFieldDef| visit_ascribed_node(node, ""))
            .visit(|node: &ast::EnumVariant| Some(node.name()?.text().to_string()))
            .accept(&node)?
    }
