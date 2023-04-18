// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

use std::path::PathBuf;
use std::sync::Arc;

use dir::default_data_path;
use ethcore::client::{Client, BlockChainClient, BlockId};
use ethcore::transaction::{Transaction, Action};
use ethsync::LightSync;
use futures::{future, IntoFuture, Future, BoxFuture};
use futures_cpupool::CpuPool;
use hash_fetch::fetch::Client as FetchClient;
use hash_fetch::urlhint::ContractClient;
use helpers::replace_home;
use light::client::Client as LightClient;
use light::on_demand::{self, OnDemand};
use rpc;
use rpc_apis::SignerService;
use parity_reactor;
use util::{Bytes, Address};

#[derive(Debug, PartialEq, Clone)]
pub struct Configuration {
	pub enabled: bool,
	pub ntp_server: String,
	pub dapps_path: PathBuf,
	pub extra_dapps: Vec<PathBuf>,
	pub extra_embed_on: Vec<(String, u16)>,
}

impl Default for Configuration {
	fn default() -> Self {
		let data_dir = default_data_path();
		Configuration {
			enabled: true,
			ntp_server: "pool.ntp.org:123".into(),
			dapps_path: replace_home(&data_dir, "$BASE/dapps").into(),
			extra_dapps: vec![],
			extra_embed_on: vec![],
		}
	}
}

impl Configuration {
	pub fn address(&self, address: Option<(String, u16)>) -> Option<(String, u16)> {
		match self.enabled {
			true => address,
			false => None,
		}
	}
}

/// Registrar implementation of the full client.
pub struct FullRegistrar {
	/// Handle to the full client.
	pub client: Arc<Client>,
}

impl ContractClient for FullRegistrar {
	fn registrar(&self) -> Result<Address, String> {
		self.client.additional_params().get("registrar")
			 .ok_or_else(|| "Registrar not defined.".into())
			 .and_then(|registrar| {
				 registrar.parse().map_err(|e| format!("Invalid registrar address: {:?}", e))
			 })
	}

	fn call(&self, address: Address, data: Bytes) -> BoxFuture<Bytes, String> {
		self.client.call_contract(BlockId::Latest, address, data)
			.into_future()
			.boxed()
	}
}

/// Registrar implementation for the light client.
pub struct LightRegistrar {
	/// The light client.
	pub client: Arc<LightClient>,
	/// Handle to the on-demand service.
	pub on_demand: Arc<OnDemand>,
	/// Handle to the light network service.
	pub sync: Arc<LightSync>,
}

impl ContractClient for LightRegistrar {
	fn registrar(&self) -> Result<Address, String> {
		self.client.engine().additional_params().get("registrar")
			 .ok_or_else(|| "Registrar not defined.".into())
			 .and_then(|registrar| {
				 registrar.parse().map_err(|e| format!("Invalid registrar address: {:?}", e))
			 })
	}

	fn call(&self, address: Address, data: Bytes) -> BoxFuture<Bytes, String> {
		let (header, env_info) = (self.client.best_block_header(), self.client.latest_env_info());

		let maybe_future = self.sync.with_context(move |ctx| {
			self.on_demand
				.request(ctx, on_demand::request::TransactionProof {
					tx: Transaction {
						nonce: self.client.engine().account_start_nonce(header.number()),
						action: Action::Call(address),
						gas: 50_000_000.into(),
						gas_price: 0.into(),
						value: 0.into(),
						data: data,
					}.fake_sign(Address::default()),
					header: header.into(),
					env_info: env_info,
					engine: self.client.engine().clone(),
				})
				.expect("No back-references; therefore all back-refs valid; qed")
				.then(|res| match res {
					Ok(Ok(executed)) => Ok(executed.output),
					Ok(Err(e)) => Err(format!("Failed to execute transaction: {}", e)),
					Err(_) => Err(format!("On-demand service dropped request unexpectedly.")),
				})
		});

		match maybe_future {
			Some(fut) => fut.boxed(),
			None => future::err("cannot query registry: network disabled".into()).boxed(),
		}
	}
}

// TODO: light client implementation forwarding to OnDemand and waiting for future
// to resolve.
#[derive(Clone)]
pub struct Dependencies {
	pub sync_status: Arc<SyncStatus>,
	pub contract_client: Arc<ContractClient>,
	pub remote: parity_reactor::TokioRemote,
	pub pool: CpuPool,
	pub fetch: FetchClient,
	pub signer: Arc<SignerService>,
	pub ui_address: Option<(String, u16)>,
}

pub fn new(configuration: Configuration, deps: Dependencies) -> Result<Option<Middleware>, String> {
	if !configuration.enabled {
		return Ok(None);
	}

	server::dapps_middleware(
		deps,
		&configuration.ntp_server,
		configuration.dapps_path,
		configuration.extra_dapps,
		rpc::DAPPS_DOMAIN,
		configuration.extra_embed_on,
	).map(Some)
}

pub fn new_ui(enabled: bool, ntp_server: &str, deps: Dependencies) -> Result<Option<Middleware>, String> {
	if !enabled {
		return Ok(None);
	}

	server::ui_middleware(
		deps,
		ntp_server,
		rpc::DAPPS_DOMAIN,
	).map(Some)
}

pub use self::server::{SyncStatus, Middleware, service};

#[cfg(not(feature = "dapps"))]
mod server {
	use super::Dependencies;
	use std::sync::Arc;
	use std::path::PathBuf;
	use parity_rpc::{hyper, RequestMiddleware, RequestMiddlewareAction};
	use rpc_apis;

	pub trait SyncStatus {
		fn is_major_importing(&self) -> bool;
		fn peers(&self) -> (usize, usize);
	}

	pub struct Middleware;
	impl RequestMiddleware for Middleware {
		fn on_request(
			&self, _req: &hyper::server::Request<hyper::net::HttpStream>, _control: &hyper::Control
		) -> RequestMiddlewareAction {
			unreachable!()
		}
	}

	pub fn dapps_middleware(
		_deps: Dependencies,
		_ntp_server: &str,
		_dapps_path: PathBuf,
		_extra_dapps: Vec<PathBuf>,
		_dapps_domain: &str,
		_extra_embed_on: Vec<(String, u16)>,
	) -> Result<Middleware, String> {
		Err("Your Parity version has been compiled without WebApps support.".into())
	}

	pub fn ui_middleware(
		_deps: Dependencies,
		_ntp_server: &str,
		_dapps_domain: &str,
	) -> Result<Middleware, String> {
		Err("Your Parity version has been compiled without UI support.".into())
	}

	pub fn service(_: &Option<Middleware>) -> Option<Arc<rpc_apis::DappsService>> {
		None
	}
}

#[cfg(feature = "dapps")]
mod server {
	use super::Dependencies;
	use std::path::PathBuf;
	use std::sync::Arc;
	use rpc_apis;

	use parity_dapps;
	use parity_reactor;

	pub use parity_dapps::Middleware;
	pub use parity_dapps::SyncStatus;

	pub fn dapps_middleware(
		deps: Dependencies,
		ntp_server: &str,
		dapps_path: PathBuf,
		extra_dapps: Vec<PathBuf>,
		dapps_domain: &str,
		extra_embed_on: Vec<(String, u16)>,
	) -> Result<Middleware, String> {
		let signer = deps.signer;
		let parity_remote = parity_reactor::Remote::new(deps.remote.clone());
		let web_proxy_tokens = Arc::new(move |token| signer.web_proxy_access_token_domain(&token));

		Ok(parity_dapps::Middleware::dapps(
			ntp_server,
			deps.pool,
			parity_remote,
			deps.ui_address,
			extra_embed_on,
			dapps_path,
			extra_dapps,
			dapps_domain,
			deps.contract_client,
			deps.sync_status,
			web_proxy_tokens,
			deps.fetch,
		))
	}

	pub fn ui_middleware(
		deps: Dependencies,
		ntp_server: &str,
		dapps_domain: &str,
	) -> Result<Middleware, String> {
		let parity_remote = parity_reactor::Remote::new(deps.remote.clone());
		Ok(parity_dapps::Middleware::ui(
			ntp_server,
			deps.pool,
			parity_remote,
			dapps_domain,
			deps.contract_client,
			deps.sync_status,
			deps.fetch,
		))
	}

	pub fn service(middleware: &Option<Middleware>) -> Option<Arc<rpc_apis::DappsService>> {
		middleware.as_ref().map(|m| Arc::new(DappsServiceWrapper {
			endpoints: m.endpoints()
		}) as Arc<rpc_apis::DappsService>)
	}

	pub struct DappsServiceWrapper {
		endpoints: parity_dapps::Endpoints,
	}

	impl rpc_apis::DappsService for DappsServiceWrapper {
		fn list_dapps(&self) -> Vec<rpc_apis::LocalDapp> {
			self.endpoints.list()
				.into_iter()
				.map(|app| rpc_apis::LocalDapp {
					id: app.id,
					name: app.name,
					description: app.description,
					version: app.version,
					author: app.author,
					icon_url: app.icon_url,
				})
				.collect()
		}
	}
}
