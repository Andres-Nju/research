File_Code/yew/cb671ad4d2/list/list_after.rs --- 1/2 --- Rust
4 use yew::html::ChildrenRenderer;                                                                                                                           4 use yew::html::{ChildrenRenderer, NodeRef};

File_Code/yew/cb671ad4d2/list/list_after.rs --- 2/2 --- Rust
158             Variants::Header(props) => VComp::new::<ListHeader>(props, self.scope).into(),                                                               158             Variants::Header(props) => VComp::new::<ListHeader>(props, self.scope, NodeRef::default()).into(),
159             Variants::Item(props) => VComp::new::<ListItem>(props, self.scope).into(),                                                                   159             Variants::Item(props) => VComp::new::<ListItem>(props, self.scope, NodeRef::default()).into(),

