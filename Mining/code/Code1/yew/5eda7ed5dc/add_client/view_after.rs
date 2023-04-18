    fn view(&self) -> Html {
        let Self { link, client, .. } = self;
        html! {
            <>
                <div class="names">
                    <input
                        class=classes!("new-client", "firstname")
                        placeholder="First name"
                        value=client.first_name.clone()
                        oninput=link.callback(|e: InputData| Msg::UpdateFirstName(e.value))
                    />
                    <input
                        class=classes!("new-client", "lastname")
                        placeholder="Last name"
                        value=client.last_name.clone()
                        oninput=link.callback(|e: InputData| Msg::UpdateLastName(e.value))
                    />
                    <textarea
                        class=classes!("new-client", "description")
                        placeholder="Description"
                        value=client.description.clone()
                        oninput=link.callback(|e: InputData| Msg::UpdateDescription(e.value))
                    />
                </div>

                <button
                    disabled=client.first_name.is_empty() || client.last_name.is_empty()
                    onclick=link.callback(|_| Msg::Add)
                >
                    { "Add New" }
                </button>
                <button onclick=link.callback(|_| Msg::Abort)>
                    { "Go Back" }
                </button>
            </>
        }
    }
