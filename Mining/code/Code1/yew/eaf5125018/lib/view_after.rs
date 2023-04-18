    fn view(&self) -> Html {
        html! {
            <div>
                <textarea oninput=self.link.callback(move |input: InputData| Msg::Payload(input.value))
                    style="font-family: 'Monaco' monospace;"
                    value={ &self.payload }>
                </textarea>
                <button onclick=self.link.callback(|_| Msg::Payload(get_payload()))>
                    { "Get the payload!" }
                </button>
                <button onclick=self.link.callback(|_| Msg::AsyncPayload) >
                    { "Get the payload later!" }
                </button>
                <p style="font-family: 'Monaco', monospace;">
                    { nbsp(self.debugged_payload.as_str()) }
                </p>
            </div>
        }
    }
