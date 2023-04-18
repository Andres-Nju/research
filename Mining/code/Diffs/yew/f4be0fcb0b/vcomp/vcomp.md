File_Code/yew/f4be0fcb0b/vcomp/vcomp_after.rs --- 1/3 --- Rust
468         node.apply(&scope, &parent, None, None);                                                                                                         468         node.apply(&scope, &parent, NodeRef::default(), None);

File_Code/yew/f4be0fcb0b/vcomp/vcomp_after.rs --- 2/3 --- Rust
523     use crate::{Children, Component, ComponentLink, Html, Properties, Renderable, ShouldRender};                                                         523     use crate::{Children, Component, ComponentLink, Html, Properties, ShouldRender};

File_Code/yew/f4be0fcb0b/vcomp/vcomp_after.rs --- 3/3 --- Rust
...                                                                                                                                                          564             html! {
564             self.props.children.render()                                                                                                                 565                 <>{ self.props.children.clone() }</>
                                                                                                                                                             566             }

