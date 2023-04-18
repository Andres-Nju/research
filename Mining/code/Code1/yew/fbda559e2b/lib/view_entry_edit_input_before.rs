    fn view_entry_edit_input(&self, (idx, entry): (usize, &Entry)) -> Html {
        if entry.editing {
            html! {
                <input class="edit"
                       type="text"
                       value=&entry.description
                       oninput=self.link.callback(|e: InputData| Msg::UpdateEdit(e.value))
                       onblur=self.link.callback(move |_| Msg::Edit(idx))
                       onkeypress=self.link.callback(move |e: KeyboardEvent| {
                          if e.key() == "Enter" { Msg::Edit(idx) } else { Msg::Nope }
                       }) />
            }
        } else {
            html! { <input type="hidden" /> }
        }
    }
