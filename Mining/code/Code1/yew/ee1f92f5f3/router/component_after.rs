fn component() -> Html {
    let history = use_history().unwrap();

    let switch = Switch::render(move |routes| {
        let history_clone = history.clone();
        let replace_route = Callback::from(move |_| {
            history_clone
                .replace_with_query(
                    Routes::No { id: 2 },
                    Query {
                        foo: "bar".to_string(),
                    },
                )
                .unwrap();
        });

        let history_clone = history.clone();
        let push_route = Callback::from(move |_| {
            history_clone
                .push_with_query(
                    Routes::No { id: 3 },
                    Query {
                        foo: "baz".to_string(),
                    },
                )
                .unwrap();
        });

        match routes {
            Routes::Home => html! {
                <>
                    <div id="result">{"Home"}</div>
                    <button onclick={replace_route}>{"replace a route"}</button>
                </>
            },
            Routes::No { id } => html! {
                <>
                    <No id={*id} />
                    <button onclick={push_route}>{"push a route"}</button>
                </>
            },
            Routes::NotFound => html! { <div id="result">{"404"}</div> },
        }
    });

    html! {
        <Switch<Routes> render={switch} />
    }
}
