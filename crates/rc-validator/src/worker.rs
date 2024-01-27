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

use crate::{Command, DkgSignature, DkgVerifyingKey, Identifier};
use anyhow::{Ok as AnyOk, Result};
use cumulus_primitives_core::relay_chain::ValidatorId;
use futures::{
	channel::{mpsc, oneshot},
	stream::StreamExt,
};
use log::{debug, error};
use rc_validator_network::{Arc, Service as ValidatorNetworkService};
use redot_core_primitives::crypto::{DkgMessage, FrostDkg, SignMessage};
use serde::Serialize;

// Represents different types of responses that can be sent back from the Worker.
enum QueryResultSender {
	RotateKey(oneshot::Sender<Result<DkgVerifyingKey>>),
	Sign(oneshot::Sender<Result<DkgSignature>>),
}

// Macro to handle sending responses back to the requestor.
macro_rules! handle_send {
	($sender_variant:ident, $msg:expr, $result:expr) => {
		if let Some(QueryResultSender::$sender_variant(ch)) = $msg {
			if ch.send($result).is_err() {
				debug!("Failed to send result");
			}
		}
	};
}

/// The Worker struct represents a worker in the network that handles various tasks.
///
/// It processes commands and messages related to DKG (Distributed Key Generation) and signing,
/// interacting with the FrostDkg protocol for cryptographic operations.
pub struct Worker {
	network: Arc<ValidatorNetworkService>,
	frost_dkg: FrostDkg,
	command_receiver: mpsc::Receiver<Command>,
	dkg_sender: Option<QueryResultSender>,
	sign_sender: Option<QueryResultSender>,
}

// Topics for DKG and signing messages.
const DKG_TOPIC: &str = "dkg_topic";
const SIGN_TOPIC: &str = "sign_topic";

impl Worker {
	/// Creates a new Worker instance.
	///
	/// # Arguments
	///
	/// * `network` - Shared reference to the ValidatorNetworkService.
	/// * `validator_id` - The unique identifier of the validator.
	/// * `command_receiver` - Receiver for commands to be processed by the worker.
	///
	/// # Returns
	///
	/// A result containing either the new Worker instance or an error.
	pub fn new(
		network: Arc<ValidatorNetworkService>,
		validator_id: ValidatorId,
		command_receiver: mpsc::Receiver<Command>,
	) -> Result<Self> {
		let id = Identifier::derive(validator_id.to_string().as_bytes())?;
		let frost_dkg = FrostDkg::new(id);
		AnyOk(Self { network, frost_dkg, command_receiver, dkg_sender: None, sign_sender: None })
	}

	/// Main loop of the worker, handling incoming DKG and signing messages, and commands.
	pub async fn run(&mut self) -> Result<()> {
		let mut dkg_receiver = self.network.subscribe(DKG_TOPIC).await?.receiver;
		let mut sign_receiver = self.network.subscribe(SIGN_TOPIC).await?.receiver;

		loop {
			futures::select! {
				dkg_message = dkg_receiver.select_next_some() => {
					self.handle_dkg_message(dkg_message.into()).await;
				},
				sign_message = sign_receiver.select_next_some() => {
					self.handle_sign_message(sign_message.into()).await;
				},
				command = self.command_receiver.select_next_some() => {
					self.handle_command(command).await;
				},
			}
		}
	}

	// Handles commands received by the worker.
	//
	// Processes various commands like key rotation, signing, setup, and validator management.
	async fn handle_command(&mut self, command: Command) {
		match command {
			Command::RotateKey { sender } => {
				self.start_dkg().await;
				self.dkg_sender = Some(QueryResultSender::RotateKey(sender));
			},
			Command::Sign { message, sender } => {
				if self.sign_sender.is_some() {
					if sender
						.send(Err(anyhow::anyhow!("Another sign request is in progress")))
						.is_err()
					{
						debug!("Failed to send result");
					}
				} else {
					self.start_sign(message.as_slice()).await;
					self.sign_sender = Some(QueryResultSender::Sign(sender));
				}
			},
			Command::Setup { nt, sender } => {
				let result = self.frost_dkg.set_nt(nt.0, nt.1);
				if sender.send(result).is_err() {
					debug!("Failed to send Setup result");
				}
			},
			Command::RemoveValidators { validators, sender } => {
				let result = self.network.remove_validators(validators).await;
				if sender.send(result).is_err() {
					debug!("Failed to send result for RemoveValidators command");
				}
			},
			Command::AddValidators { validators, sender } => {
				let result = self.network.new_validators(validators).await;
				if sender.send(result).is_err() {
					debug!("Failed to send result for AddValidators command");
				}
			},
		}
	}

    // Processes DKG-related messages received by the worker.
    //
    // Handles different stages of the DKG process including part1 and part2 messages.
	async fn handle_dkg_message(&mut self, message: Vec<u8>) {
		match serde_json::from_slice::<DkgMessage>(&message) {
			Ok(message) => match message {
				DkgMessage::DkgPart1(dkg_part1_message) => {
					match self.frost_dkg.dkg_part1(dkg_part1_message) {
						Ok(msg) => {
							if let Err(e) = self.serialize_and_publish(DKG_TOPIC, &msg).await {
								error!("Failed to publish DKG Part1 message: {}", e);
							}
						},
						Err(e) => error!("Error in DKG Part1 processing: {}", e),
					}
				},
				DkgMessage::DkgPart2(dkg_part2_message) => {
					match self.frost_dkg.dkg_part2(dkg_part2_message) {
						Ok(msg) => {
							if let Some(key) = msg {
								handle_send!(RotateKey, self.dkg_sender.take(), Ok(key));
							} else {
								if let Err(e) = self.serialize_and_publish(DKG_TOPIC, &msg).await {
									error!("Failed to publish DKG Part2 message: {}", e);
								}
							}
						},
						Err(e) => {
							handle_send!(RotateKey, self.dkg_sender.take(), Err(e.into()));
							error!("Error in DKG Part2 processing.");
						},
					}
				},
			},
			Err(e) => error!("Failed to deserialize DKG message: {}", e),
		}
	}

	// Processes signing-related messages received by the worker.
    //
    // Handles different stages of the signing process including part1 and part2 messages.
    async fn handle_sign_message(&mut self, message: Vec<u8>) {
		match serde_json::from_slice::<SignMessage>(&message) {
			Ok(message) => match message {
				SignMessage::SignPart1(sign_part1_message) => {
					match self.frost_dkg.sign_part1(sign_part1_message.clone()) {
						Ok(msg) => {
							if let Err(e) = self.serialize_and_publish(SIGN_TOPIC, &msg).await {
								error!("Failed to publish Sign Part1 message: {}", e);
							}
						},
						Err(e) => error!("Error in Sign Part1 processing: {}", e),
					}

					let message = SignMessage::SignPart1(sign_part1_message);

					if let Err(e) = self.serialize_and_publish(SIGN_TOPIC, &message).await {
						error!("Failed to publish Sign Part1 message: {}", e);
					}
				},
				SignMessage::SignPart2(sign_part2_message) => {
					match self.frost_dkg.sign_part2(sign_part2_message.clone()) {
						Ok(signature) => {
							if let Some(sign) = signature {
								handle_send!(Sign, self.sign_sender.take(), Ok(sign));
							}
						},
						Err(e) => {
							handle_send!(Sign, self.sign_sender.take(), Err(e.into()));
						},
					}
				},
			},
			Err(e) => error!("Failed to deserialize SignMessage: {}", e),
		}
	}

	// Initiates the DKG process.
    //
    // Starts the DKG process by generating and publishing the first part of the DKG message.
    async fn start_dkg(&mut self) {
		match self.frost_dkg.start_dkg() {
			Ok(msg) => {
				if let Err(e) = self.serialize_and_publish(DKG_TOPIC, &msg).await {
					error!("Failed to publish DKG Part1 message: {}", e);
				}
			},
			Err(e) => error!("Error in DKG Part1 processing: {}", e),
		}
	}

	// Initiates the signing process for a given message.
    //
    // Starts the signing process by generating and publishing the first part of the signing message.
    async fn start_sign(&mut self, message: &[u8]) {
		match self.frost_dkg.start_sign(message) {
			Ok(msg) => {
				if let Err(e) = self.serialize_and_publish(SIGN_TOPIC, &msg).await {
					error!("Failed to publish Sign Part1 message: {}", e);
				}
			},
			Err(e) => error!("Error in Sign Part1 processing: {}", e),
		}
	}

	// Serializes and publishes a given message to a specified topic.
    //
    // # Arguments
    //
    // * `topic` - The topic to which the message will be published.
    // * `message` - The message to be serialized and published.
    //
    // # Returns
    //
    // A result indicating success or failure of the operation.
	async fn serialize_and_publish<T: Serialize>(&self, topic: &str, message: &T) -> Result<()> {
		match serde_json::to_vec(message) {
			Ok(encoded_msg) => self.network.publish(topic, encoded_msg).await.map_err(Into::into),
			Err(e) => {
				error!("Failed to serialize message: {}", e);
				Err(e.into())
			},
		}
	}
}
