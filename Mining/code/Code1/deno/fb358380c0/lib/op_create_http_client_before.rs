pub fn op_create_http_client<FP>(
  state: &mut OpState,
  args: Value,
  _zero_copy: &mut [ZeroCopyBuf],
) -> Result<Value, AnyError>
where
  FP: FetchPermissions + 'static,
{
  #[derive(Deserialize, Default, Debug)]
  #[serde(rename_all = "camelCase")]
  #[serde(default)]
  struct CreateHttpClientOptions {
    ca_file: Option<String>,
    ca_data: Option<String>,
  }

  let args: CreateHttpClientOptions = serde_json::from_value(args)?;

  if let Some(ca_file) = args.ca_file.clone() {
    let permissions = state.borrow::<FP>();
    permissions.check_read(&PathBuf::from(ca_file))?;
  }

  let client =
    create_http_client(args.ca_file.as_deref(), args.ca_data.as_deref())
      .unwrap();

  let rid = state.resource_table.add(HttpClientResource::new(client));
  Ok(json!(rid))
}

/// Create new instance of async reqwest::Client. This client supports
/// proxies and doesn't follow redirects.
fn create_http_client(
  ca_file: Option<&str>,
  ca_data: Option<&str>,
) -> Result<Client, AnyError> {
  let mut builder = Client::builder().redirect(Policy::none()).use_rustls_tls();
  if let Some(ca_data) = ca_data {
    let ca_data_vec = ca_data.as_bytes().to_vec();
    let cert = reqwest::Certificate::from_pem(&ca_data_vec)?;
    builder = builder.add_root_certificate(cert);
  } else if let Some(ca_file) = ca_file {
    let mut buf = Vec::new();
    File::open(ca_file)?.read_to_end(&mut buf)?;
    let cert = reqwest::Certificate::from_pem(&buf)?;
    builder = builder.add_root_certificate(cert);
  }
  builder
    .build()
    .map_err(|_| deno_core::error::generic_error("Unable to build http client"))
}
