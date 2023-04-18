    fn next(&mut self) -> Option<Self::Item> {
        let mut group = self.previous.clone();
        let mut current_count = 0;

        if group.is_empty() {
            loop {
                let item = self.input.next();

                match item {
                    Some(v) => {
                        group.push(v);

                        current_count += 1;
                        if current_count >= self.group_size {
                            break;
                        }
                    }
                    None => return None,
                }
            }
        } else {
            // our historic buffer is already full, so stride instead

            loop {
                let item = self.input.next();

                match item {
                    Some(v) => {
                        group.push(v);

                        current_count += 1;
                        if current_count >= self.stride {
                            break;
                        }
                    }
                    None => return None,
                }
            }

            group = group[current_count..].to_vec();
        }

        if group.is_empty() || current_count == 0 {
            return None;
        }

        self.previous = group.clone();

        Some(Value::List {
            vals: group,
            span: self.span,
        })
    }
