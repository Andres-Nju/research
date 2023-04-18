	fn on_request(&mut self, req: server::Request<HttpStream>) -> Next {

		// Choose proper handler depending on path / domain
		let url = extract_url(&req);
		let endpoint = extract_endpoint(&url);
		let is_utils = endpoint.1 == SpecialEndpoint::Utils;

		trace!(target: "dapps", "Routing request to {:?}. Details: {:?}", url, req);

		// Validate Host header
		if let Some(ref hosts) = self.allowed_hosts {
			trace!(target: "dapps", "Validating host headers against: {:?}", hosts);
			let is_valid = is_utils || host_validation::is_valid(&req, hosts, self.endpoints.keys().cloned().collect());
			if !is_valid {
				debug!(target: "dapps", "Rejecting invalid host header.");
				self.handler = host_validation::host_invalid_response();
				return self.handler.on_request(req);
			}
		}

		trace!(target: "dapps", "Checking authorization.");
		// Check authorization
		let auth = self.authorization.is_authorized(&req);
		if let Authorized::No(handler) = auth {
			debug!(target: "dapps", "Authorization denied.");
			self.handler = handler;
			return self.handler.on_request(req);
		}

		let control = self.control.take().expect("on_request is called only once; control is always defined at start; qed");
		debug!(target: "dapps", "Handling endpoint request: {:?}", endpoint);
		self.handler = match endpoint {
			// First check special endpoints
			(ref path, ref endpoint) if self.special.contains_key(endpoint) => {
				trace!(target: "dapps", "Resolving to special endpoint.");
				self.special.get(endpoint)
					.expect("special known to contain key; qed")
					.to_async_handler(path.clone().unwrap_or_default(), control)
			},
			// Then delegate to dapp
			(Some(ref path), _) if self.endpoints.contains_key(&path.app_id) => {
				trace!(target: "dapps", "Resolving to local/builtin dapp.");
				self.endpoints.get(&path.app_id)
					.expect("special known to contain key; qed")
					.to_async_handler(path.clone(), control)
			},
			// Try to resolve and fetch the dapp
			(Some(ref path), _) if self.fetch.contains(&path.app_id) => {
				trace!(target: "dapps", "Resolving to fetchable content.");
				self.fetch.to_async_handler(path.clone(), control)
			},
			// NOTE [todr] /home is redirected to home page since some users may have the redirection cached
			// (in the past we used 301 instead of 302)
			// It should be safe to remove it in (near) future.
			//
			// 404 for non-existent content
			(Some(ref path), _) if *req.method() == hyper::Method::Get && path.app_id != "home" => {
				trace!(target: "dapps", "Resolving to 404.");
				Box::new(ContentHandler::error(
					StatusCode::NotFound,
					"404 Not Found",
					"Requested content was not found.",
					None,
					self.signer_address.clone(),
				))
			},
			// Redirect any other GET request to signer.
			_ if *req.method() == hyper::Method::Get => {
				if let Some(signer_address) = self.signer_address.clone() {
					trace!(target: "dapps", "Redirecting to signer interface.");
					Redirection::boxed(&format!("http://{}", address(signer_address)))
				} else {
					trace!(target: "dapps", "Signer disabled, returning 404.");
					Box::new(ContentHandler::error(
						StatusCode::NotFound,
						"404 Not Found",
						"Your homepage is not available when Trusted Signer is disabled.",
						Some("You can still access dapps by writing a correct address, though. Re-enable Signer to get your homepage back."),
						self.signer_address.clone(),
					))
				}
			},
			// RPC by default
			_ => {
				trace!(target: "dapps", "Resolving to RPC call.");
				self.special.get(&SpecialEndpoint::Rpc)
					.expect("RPC endpoint always stored; qed")
					.to_async_handler(EndpointPath::default(), control)
			}
		};

		// Delegate on_request to proper handler
		self.handler.on_request(req)
	}
