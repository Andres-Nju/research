    fn parse_ids(&mut self, file: &Path, contents: &str, errors: &mut bool) {
        if self.ids.is_empty() {
            with_attrs_in_source(contents, " id", |fragment, i, _| {
                let frag = fragment.trim_left_matches("#").to_owned();
                let encoded = small_url_encode(&frag);
                if !self.ids.insert(frag) {
                    *errors = true;
                    println!("{}:{}: id is not unique: `{}`", file.display(), i, fragment);
                }
                // Just in case, we also add the encoded id.
                self.ids.insert(encoded);
            });
        }
    }
