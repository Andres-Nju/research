        fn view(&self) -> Html {
            html! {
                <>{ self.props.children.clone() }</>
            }
        }
