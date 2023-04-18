pub fn initial_read_me() -> String {
  let document_content = include_str!("READ_ME.json");
  return document_content.to_string();
}
