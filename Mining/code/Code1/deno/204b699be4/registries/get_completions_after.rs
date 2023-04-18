  pub async fn get_completions(
    &self,
    current_specifier: &str,
    offset: usize,
    range: &lsp::Range,
    state_snapshot: &language_server::StateSnapshot,
  ) -> Option<Vec<lsp::CompletionItem>> {
    if let Ok(specifier) = Url::parse(current_specifier) {
      let origin = base_url(&specifier);
      let origin_len = origin.chars().count();
      if offset >= origin_len {
        if let Some(registries) = self.origins.get(&origin) {
          let path = &specifier[Position::BeforePath..];
          let path_offset = offset - origin_len;
          let mut completions = HashMap::<String, lsp::CompletionItem>::new();
          let mut did_match = false;
          for registry in registries {
            let tokens = parse(&registry.schema, None)
              .map_err(|e| {
                error!(
                  "Error parsing registry schema for origin \"{}\". {}",
                  origin, e
                );
              })
              .ok()?;
            let mut i = tokens.len();
            let last_key_name =
              StringOrNumber::String(tokens.iter().last().map_or_else(
                || "".to_string(),
                |t| {
                  if let Token::Key(key) = t {
                    if let StringOrNumber::String(s) = &key.name {
                      return s.clone();
                    }
                  }
                  "".to_string()
                },
              ));
            loop {
              let matcher = Matcher::new(&tokens[..i], None)
                .map_err(|e| {
                  error!(
                    "Error creating matcher for schema for origin \"{}\". {}",
                    origin, e
                  );
                })
                .ok()?;
              if let Some(match_result) = matcher.matches(path) {
                did_match = true;
                let completor_type =
                  get_completor_type(path_offset, &tokens, &match_result);
                match completor_type {
                  Some(CompletorType::Literal(s)) => self.complete_literal(
                    s,
                    &mut completions,
                    current_specifier,
                    offset,
                    range,
                  ),
                  Some(CompletorType::Key(k, p)) => {
                    let maybe_url = registry.variables.iter().find_map(|v| {
                      if k.name == StringOrNumber::String(v.key.clone()) {
                        Some(v.url.as_str())
                      } else {
                        None
                      }
                    });
                    if let Some(url) = maybe_url {
                      if let Some(items) = self
                        .get_variable_items(url, &tokens, &match_result)
                        .await
                      {
                        let end = if p.is_some() { i + 1 } else { i };
                        let compiler = Compiler::new(&tokens[..end], None);
                        for (idx, item) in items.into_iter().enumerate() {
                          let label = if let Some(p) = &p {
                            format!("{}{}", p, item)
                          } else {
                            item.clone()
                          };
                          let kind = if k.name == last_key_name {
                            Some(lsp::CompletionItemKind::File)
                          } else {
                            Some(lsp::CompletionItemKind::Folder)
                          };
                          let mut params = match_result.params.clone();
                          params.insert(
                            k.name.clone(),
                            StringOrVec::from_str(&item, &k),
                          );
                          let path =
                            compiler.to_path(&params).unwrap_or_default();
                          let mut item_specifier = Url::parse(&origin).ok()?;
                          item_specifier.set_path(&path);
                          let full_text = item_specifier.as_str();
                          let text_edit = Some(lsp::CompletionTextEdit::Edit(
                            lsp::TextEdit {
                              range: *range,
                              new_text: full_text.to_string(),
                            },
                          ));
                          let command = if k.name == last_key_name
                            && !state_snapshot
                              .sources
                              .contains_key(&item_specifier)
                          {
                            Some(lsp::Command {
                              title: "".to_string(),
                              command: "deno.cache".to_string(),
                              arguments: Some(vec![json!([item_specifier])]),
                            })
                          } else {
                            None
                          };
                          let detail = Some(format!("({})", k.name));
                          let filter_text = Some(full_text.to_string());
                          let sort_text = Some(format!("{:0>10}", idx + 1));
                          completions.insert(
                            item,
                            lsp::CompletionItem {
                              label,
                              kind,
                              detail,
                              sort_text,
                              filter_text,
                              text_edit,
                              command,
                              ..Default::default()
                            },
                          );
                        }
                      }
                    }
                  }
                  None => (),
                }
                break;
              }
              i -= 1;
              // If we have fallen though to the first token, and we still
              // didn't get a match, but the first token is a string literal, we
              // need to suggest the string literal.
              if i == 0 {
                if let Token::String(s) = &tokens[i] {
                  if s.starts_with(path) {
                    let label = s.to_string();
                    let kind = Some(lsp::CompletionItemKind::Folder);
                    let mut url = specifier.clone();
                    url.set_path(s);
                    let full_text = url.as_str();
                    let text_edit =
                      Some(lsp::CompletionTextEdit::Edit(lsp::TextEdit {
                        range: *range,
                        new_text: full_text.to_string(),
                      }));
                    let filter_text = Some(full_text.to_string());
                    completions.insert(
                      s.to_string(),
                      lsp::CompletionItem {
                        label,
                        kind,
                        filter_text,
                        sort_text: Some("1".to_string()),
                        text_edit,
                        ..Default::default()
                      },
                    );
                  }
                }
                break;
              }
            }
          }
          // If we return None, other sources of completions will be looked for
          // but if we did at least match part of a registry, we should send an
          // empty vector so that no-completions will be sent back to the client
          return if completions.is_empty() && !did_match {
            None
          } else {
            Some(completions.into_iter().map(|(_, i)| i).collect())
          };
        }
      }
    }

    self.get_origin_completions(current_specifier, range)
  }
