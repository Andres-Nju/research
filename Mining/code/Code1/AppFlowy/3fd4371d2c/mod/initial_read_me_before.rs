pub fn initial_read_me() -> String {
  let document_content = include_str!("READ_ME.json");
  let transaction = make_transaction_from_document_content(document_content).unwrap();
  transaction.to_json().unwrap()
}
