    fn view(&self) -> Html {
        let flag = self.by_chunks;
        html! {
            <div>
                <div>
                    <input type="file" multiple=true onchange=|value| {
                            let mut result = Vec::new();
                            if let ChangeData::Files(files) = value {
                                result.extend(files);
                            }
                            Msg::Files(result, flag)
                        }/>
                </div>
                <div>
                    <label>{ "By chunks" }</label>
                    <input type="checkbox" checked=flag onclick=self.link.callback(|_| Msg::ToggleByChunks) />
                </div>
                <ul>
                    { for self.files.iter().map(|f| self.view_file(f)) }
                </ul>
            </div>
        }
    }
