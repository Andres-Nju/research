    fn into(self) -> VNode<List> {
        match self.props {
            Variants::Header(props) => VComp::new::<ListHeader>(props, self.scope, NodeRef::default()).into(),
            Variants::Item(props) => VComp::new::<ListItem>(props, self.scope, NodeRef::default()).into(),
        }
    }
