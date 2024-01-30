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

use crate::{Command, DkgSignature, DkgVerifyingKey};
use anyhow::{Context, Result};
use cumulus_primitives_core::relay_chain::ValidatorId;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt,
};

use std::fmt::Debug;

/// `Service` acts as an intermediary for interacting with a Worker. It handles requests and
/// facilitates communication between the service and the worker through a message-passing mechanism.
/// 
/// This struct is primarily used for sending various commands to the worker, such as key rotation,
/// starting a signing process, setting up validator network parameters, and managing validators.
#[derive(Clone)]
pub struct Service {
    // Channel sender used to send commands to the worker.
    to_worker: mpsc::Sender<Command>,
}

impl Debug for Service {
    /// Provides a custom formatter for the `Service` struct, aiding in debugging by giving a
    /// human-readable representation of the service. This method outputs the struct's name, 
    /// "ValidatorNetworkService", for simplicity.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValidatorNetworkService").finish()
    }
}

impl Service {
    /// Constructs a new instance of `Service`.
    ///
    /// # Arguments
    ///
    /// * `to_worker` - A sender channel used for sending commands to the worker.
    pub(crate) fn new(to_worker: mpsc::Sender<Command>) -> Self {
        Self { to_worker }
    }

    /// Initiates a key rotation process, resulting in a new verifier public key.
    ///
    /// This method sends a `RotateKey` command to the worker and awaits the response.
    ///
    /// # Returns
    ///
    /// A `Result` which, on success, contains the new `DkgVerifyingKey`.
    pub async fn rotate_key(&self) -> Result<DkgVerifyingKey> {
        let (sender, receiver) = oneshot::channel();
        self.to_worker
            .clone()
            .send(Command::RotateKey { sender })
            .await
            .context("Failed to send command to worker")?;
        receiver.await.context("Failed to receive response from worker")?
    }

    /// Starts a signing service and returns a signature.
    ///
    /// This method sends a `Sign` command with the provided message to the worker and waits for the signature.
    ///
    /// # Arguments
    ///
    /// * `message` - A byte slice representing the message to be signed.
    ///
    /// # Returns
    ///
    /// A `Result` which, on success, contains the `DkgSignature`.
    pub async fn start_signing(&self, message: &[u8]) -> Result<DkgSignature> {
        let (sender, receiver) = oneshot::channel();
        self.to_worker
            .clone()
            .send(Command::Sign { message: message.to_vec(), sender })
            .await
            .context("Failed to send command to worker")?;
        receiver.await.context("Failed to receive response from worker")?
    }

    /// Sets up the validator network with specified threshold and total number of participants.
    ///
    /// # Arguments
    ///
    /// * `nt` - A tuple (u16, u16) where the first element is the threshold and the second is the total number of participants.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub async fn setup(&self, nt: (u16, u16)) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.to_worker
            .clone()
            .send(Command::Setup { nt, sender })
            .await
            .context("Failed to send command to worker")?;
        receiver.await.context("Failed to receive response from worker")?
    }

    /// Removes validators from the network. These validators will no longer be part of the validator network.
    ///
    /// # Arguments
    ///
    /// * `validators` - A vector of `ValidatorId` representing the validators to be removed.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub async fn remove_validators(&self, validators: Vec<ValidatorId>) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.to_worker
            .clone()
            .send(Command::RemoveValidators { validators, sender })
            .await
            .context("Failed to send command to worker")?;
        receiver.await.context("Failed to receive response from worker")?
    }

    /// Adds new validators to the network. These validators will be included in the validator network.
    ///
    /// # Arguments
    ///
    /// * `validators` - A vector of `ValidatorId` representing the validators to be added.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the operation.
    pub async fn add_validators(&self, validators: Vec<ValidatorId>) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.to_worker
            .clone()
            .send(Command::AddValidators { validators, sender })
            .await
            .context("Failed to send command to worker")?;
        receiver.await.context("Failed to receive response from worker")?
    }
}

