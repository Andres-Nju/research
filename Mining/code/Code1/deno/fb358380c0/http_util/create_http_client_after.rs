pub fn create_http_client(
  user_agent: String,
  ca_data: Option<Vec<u8>>,
) -> Result<Client, AnyError> {
  let mut headers = HeaderMap::new();
  headers.insert(USER_AGENT, user_agent.parse().unwrap());
  let mut builder = Client::builder()
    .redirect(Policy::none())
    .default_headers(headers)
    .use_rustls_tls();

  if let Some(ca_data) = ca_data {
    let cert = reqwest::Certificate::from_pem(&ca_data)?;
    builder = builder.add_root_certificate(cert);
  }

  builder
    .build()
    .map_err(|e| generic_error(format!("Unable to build http client: {}", e)))
}
