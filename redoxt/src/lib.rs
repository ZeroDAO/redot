// Copyright 2023 ZeroDAO

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Result;
use codec::Encode;
use frost_ed25519::{Signature as DkgSignature, VerifyingKey};
use redot::runtime_types::bounded_collections::weak_bounded_vec::WeakBoundedVec;
pub use subxt::PolkadotConfig as RedotConfig;
use subxt::{
	ext::scale_encode::EncodeAsType,
	utils::{AccountId32, MultiAddress, MultiSignature},
	OnlineClient,
};
use subxt_signer::sr25519::{
	dev::{self},
	Keypair,
};

// Load the runtime metadata from the provided path.
#[subxt::subxt(runtime_metadata_path = "redoxt_metadata.scale")]
pub mod redot {}

mod log;
pub use crate::log::init_logger;

mod helper;
pub use helper::*;

/// Configuration enum for Melo blockchain.
pub enum MeloConfig {}

// Define aliases for commonly used types.
pub type Signature = MultiSignature;
pub type AccountId = AccountId32;
pub type AccountIndex = u32;
pub type Address = MultiAddress<AccountId, AccountIndex>;

/// Client structure containing the API for blockchain interactions and a signer for transactions.
pub struct Client {
	pub api: OnlineClient<RedotConfig>,
	pub signer: Keypair,
}

impl Client {
	/// Update the signer for the client.
	pub fn set_signer(&mut self, signer: Keypair) {
		self.signer = signer;
	}

	/// Update the API client.
	pub fn set_client(&mut self, api: OnlineClient<RedotConfig>) {
		self.api = api;
	}

	/// Get the storage key for a given pallet and entry.
	pub fn storage_key(
		&self,
		pallet_name: &str,
		entry_name: &str,
		key: &impl EncodeAsType,
	) -> Result<Vec<u8>> {
		let address = subxt::dynamic::storage(pallet_name, entry_name, vec![key]);
		Ok(self.api.storage().address_bytes(&address)?)
	}
}

#[async_trait::async_trait]
pub trait ClientSync {
	/// Submit a new key to the blockchain.
	async fn new_key(&self, key: &VerifyingKey) -> Result<()>;

	/// Submit metadata to the blockchain.
	async fn submit_metadata<T: Encode + std::marker::Sync>(
		&self,
		metadata: &T,
		id: u32,
		nonce: u32,
		sign: &DkgSignature,
	) -> Result<()>;

	/// Rotate the key for the validator.
	async fn rotate_key(&self, key: &VerifyingKey, sign: &DkgSignature) -> Result<()>;
}

#[async_trait::async_trait]
impl ClientSync for Client {
	async fn new_key(&self, key: &VerifyingKey) -> Result<()> {
		let key_bytes = key.serialize();
		let new_key_tx = redot::tx().task().new_key(key_bytes);

		self.api
			.tx()
			.sign_and_submit_then_watch_default(&new_key_tx, &self.signer)
			.await?;
		Ok(())
	}

	async fn submit_metadata<T: Encode + std::marker::Sync>(
		&self,
		metadata: &T,
		id: u32,
		nonce: u32,
		sign: &DkgSignature,
	) -> Result<()> {
		let metadata_bytes = metadata.encode();

		let metadata_bytes = WeakBoundedVec(metadata_bytes);

		let submit_metadata_tx =
			redot::tx().task().new_metadata(id, nonce, metadata_bytes, sign.serialize());

		self.api
			.tx()
			.sign_and_submit_then_watch_default(&submit_metadata_tx, &self.signer)
			.await?;

		Ok(())
	}

	async fn rotate_key(&self, key: &VerifyingKey, sign: &DkgSignature) -> Result<()> {
		let key_bytes = key.serialize();
		let sign_bytes = sign.serialize();

		let rotate_key_tx = redot::tx().task().rotate_key(key_bytes, sign_bytes);

		self.api
			.tx()
			.sign_and_submit_then_watch_default(&rotate_key_tx, &self.signer)
			.await?;

		Ok(())
	}
}

/// A builder pattern for creating a `Client` instance.
pub struct ClientBuilder {
	pub url: String,
	pub signer: Keypair,
}

impl ClientBuilder {
	/// Constructor for `ClientBuilder`.
	pub fn new(url: &str, signer: Keypair) -> Self {
		Self { url: url.to_string(), signer }
	}

	/// Asynchronously build and return a `Client` instance.
	pub async fn build(&self) -> Result<Client> {
		let api = OnlineClient::<RedotConfig>::from_url(&self.url).await?;
		Ok(Client { api, signer: self.signer.clone() })
	}

	/// Set the URL for the API client.
	pub fn set_url(mut self, url: &str) -> Self {
		self.url = url.to_string();
		self
	}
}

// Default implementation for `ClientBuilder`.
impl Default for ClientBuilder {
	fn default() -> Self {
		Self { url: "ws://127.0.0.1:9944".to_owned(), signer: dev::alice() }
	}
}
