// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{AddrCache, Command, shared::CreatedSubscription};
use anyhow::Context;
use cumulus_primitives_core::relay_chain::ValidatorId;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt,
};
use libp2p::{futures, gossipsub::Sha256Topic, Multiaddr, PeerId};
use sp_keystore::KeystorePtr;
use std::{fmt::Debug, time::Duration};

/// The `Service` struct acts as an intermediary for interacting with the Worker. 
/// It handles requests and facilitates communication using a message passing mechanism.
#[derive(Clone)]
pub struct Service {
    // Channel sender for sending messages to the Worker.
    to_worker: mpsc::Sender<Command>,
}

impl Debug for Service {
    // Provides a human-readable representation of the Service for debugging purposes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValidatorNetworkService").finish()
    }
}

impl Service {
    /// Constructs a new `Service` instance with a given communication channel to the Worker.
    ///
    /// # Arguments
    /// * `to_worker` - A `mpsc::Sender` for sending commands to the Worker.
    pub(crate) fn new(to_worker: mpsc::Sender<Command>) -> Self {
        Self { to_worker }
    }

    /// Initiates listening on a given multi-address.
    ///
    /// # Arguments
    /// * `addr` - The `Multiaddr` to start listening on.
    ///
    /// # Returns
    /// A result indicating success or containing an error if the operation fails.
    pub async fn start_listening(&self, addr: Multiaddr) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.to_worker.clone().send(Command::StartListening { addr, sender }).await?;
        receiver.await.context("Failed receiving start listening response")?
    }

    /// Adds an address for a specific peer.
    ///
    /// # Arguments
    /// * `peer_id` - The `PeerId` of the peer.
    /// * `peer_addr` - The `Multiaddr` to add for the peer.
    ///
    /// # Returns
    /// A result indicating success or containing an error if the operation fails.
    pub async fn add_address(&self, peer_id: PeerId, peer_addr: Multiaddr) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.to_worker
            .clone()
            .send(Command::AddAddress { peer_id, peer_addr, sender })
            .await?;
        receiver.await.context("Failed receiving add address response")?
    }

    /// Removes an explicitly added peer.
    ///
    /// # Arguments
    /// * `peer_id` - The `PeerId` of the peer to be removed.
    ///
    /// # Returns
    /// A result indicating success or containing an error if the operation fails.
    pub async fn remove_explicit_peer(&self, peer_id: PeerId) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.to_worker
            .clone()
            .send(Command::RemoveExplicitPeer { peer_id, sender })
            .await?;
        receiver.await.context("Failed remove explicit peer")?
    }

    /// Informs the worker about new validators.
    ///
    /// # Arguments
    /// * `validators` - A list of new `ValidatorId`s to be added.
    ///
    /// # Returns
    /// A result indicating success.
    pub async fn new_validators(&self, validators: Vec<ValidatorId>) -> anyhow::Result<()> {
        self.to_worker.clone().send(Command::NewValidators { validators }).await?;
        Ok(())
    }

    /// Removes validators from the worker's awareness.
    ///
    /// # Arguments
    /// * `validators` - A list of `ValidatorId`s to be removed.
    ///
    /// # Returns
    /// A result indicating success.
    pub async fn remove_validators(&self, validators: Vec<ValidatorId>) -> anyhow::Result<()> {
        self.to_worker.clone().send(Command::RemoveValidators { validators }).await?;
        Ok(())
    }

    /// Subscribes to a topic.
    ///
    /// # Arguments
    /// * `topic_name` - The name of the topic to subscribe to.
    ///
    /// # Returns
    /// A result containing a `CreatedSubscription` on success, or an error if the operation fails.
    pub async fn subscribe(&self, topic_name: &str) -> anyhow::Result<CreatedSubscription> {
        let topic = Sha256Topic::new(topic_name);
        let (result_sender, result_receiver) = oneshot::channel();
        self.to_worker.clone().send(Command::Subscribe { topic, result_sender }).await?;

        match result_receiver.await.context("Failed receiving subscribe response") {
            Ok(result) => result.map_err(Into::into),
            Err(e) => Err(e.into()),
        }
    }

    /// Publishes a message to a topic.
    ///
    /// # Arguments
    /// * `topic_name` - The name of the topic to publish to.
    /// * `message` - The message to publish.
    ///
    /// # Returns
    /// A result indicating success or containing an error if the operation fails.
    pub async fn publish(&self, topic_name: &str, message: Vec<u8>) -> anyhow::Result<()> {
        let topic = Sha256Topic::new(topic_name);
        let (sender, receiver) = oneshot::channel();
        self.to_worker.clone().send(Command::Publish { topic, message, sender }).await?;
        receiver.await.context("Failed receiving publish response")?
    }
}

/// Configuration for the Validator Network service.
///
/// This struct holds configuration parameters for the validator network service, including
/// network addresses, bootstrap nodes, and retry settings.
#[derive(Clone)]
pub struct ValidatorNetworkConfig {
    /// The IP address to listen on.
    pub listen_addr: String,
    /// The port to listen on.
    pub listen_port: u16,
    /// List of bootstrap nodes to connect to.
    pub bootstrap_nodes: Vec<String>,
    /// Maximum number of retries when connecting to a node.
    pub max_retries: usize,
    /// Delay between retries when connecting to a node.
    pub retry_delay: Duration,
    /// Timeout for bootstrapping the network.
    pub bootstrap_timeout: Duration,
    /// Maximum number of parallel connections to maintain.
    pub parallel_limit: usize,
    /// The keypair of the node.
    pub key_ptr: Option<KeystorePtr>,
    /// The address cache of validators.
    pub address_cache: AddrCache,
}

impl Default for ValidatorNetworkConfig {
    fn default() -> Self {
        ValidatorNetworkConfig {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 4422,
            bootstrap_nodes: vec![],
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            bootstrap_timeout: Duration::from_secs(60),
            parallel_limit: 10,
            key_ptr: None,
            address_cache: AddrCache::new(),
        }
    }
}
