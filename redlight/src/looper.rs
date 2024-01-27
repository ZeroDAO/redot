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

use crate::DasClient;
use anyhow::{anyhow, Context};
use codec::Encode;
use log::{error, info};
use rc_validator::Service as ValidatorService;
use redoxt::{Client, ClientSync};
use std::time::Instant;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

// A simplified function for handling finalized block headers.
//
// This asynchronous function subscribes to the latest finalized block headers from a blockchain node
// and processes them to interact with a DAS (Decentralized Autonomous System) client and a validator service.
//
// # Arguments
//
// * `rpc_client` - Client instance to interact with the blockchain.
// * `message_tx` - Sender channel for sending timestamps of received messages.
// * `das_client` - Client instance to interact with the DAS system.
// * `service` - Validator service for cryptographic operations like key rotation and signing.
// * `error_sender` - Sender channel for forwarding encountered errors.
pub async fn finalized_headers(
    rpc_client: Client,
    message_tx: Sender<Instant>,
    das_client: DasClient,
    service: ValidatorService,
    error_sender: Sender<anyhow::Error>,
    // database: Arc<Mutex<SqliteDasDb>>,
) {
    // Subscribe to new blockchain headers. If it fails, log the error and return.
    let mut new_heads_sub = match rpc_client.api.blocks().subscribe_best().await {
        Ok(subscription) => {
            info!("🌐 Subscribed to finalized block headers");
            subscription
        },
        Err(e) => {
            error!("⚠️ Failed to subscribe to finalized blocks: {:?}", e);
            return;
        },
    };

    // A simple counter to keep track of processed headers.
    let mut nonce = 0;

    // Rotate the validator's key and register the new key with the blockchain.
    let init_key = service.rotate_key().await.unwrap();
    rpc_client.new_key(&init_key).await.unwrap();

    // Process each new header message as it arrives.
    while let Some(message) = new_heads_sub.next().await {
        let received_at = Instant::now();
        if let Ok(block) = message {
            let header = block.header().clone();
            let block_number = header.number;
            info!("✅ Received finalized block header #{}", block_number.clone());

            // Send the timestamp of the received header to the message channel.
            if let Err(error) = message_tx.send(received_at).await.context("Send failed") {
                error!("❌ Fail to process finalized block header: {error}");
            }

            // Retrieve the latest block information from the DAS system.
            // If it's not available or if there's an error, log it and continue or return.
            let (block_number, block_hash) = match das_client.get_latest_block() {
                Ok(Some((block_number, block_hash))) => (block_number, block_hash),
                Ok(None) => {
                    info!("No new block available yet, continuing...");
                    continue;
                },
                Err(e) => {
                    error!("❌ Fail to get latest block: {:?}", e);
                    return;
                },
            };

            // Check the data availability of the latest block from DAS.
            // If it's not available or if there's an error, log it and continue or return.
            let block_hash_hex = hex::encode(&block_hash);
            let is_available = match das_client.check_data_availability(&block_hash_hex) {
                Ok(Some(is_available)) => is_available,
                Ok(None) => {
                    info!("No new block available yet, continuing...");
                    continue;
                },
                Err(e) => {
                    error!("❌ Fail to check block availability: {:?}", e);
                    return;
                },
            };

            // Prepare and encode the metadata to be submitted to the blockchain.
            let metadata = (block_number, block_hash, is_available);
            let id = 1;
            let mut msg = metadata.encode();
            msg.extend_from_slice(&id.encode());
            msg.extend_from_slice(&nonce.encode());

            // Sign the message and submit the metadata to the blockchain.
            // Log the success or failure of the submission.
            let signature = service.start_signing(&msg.clone()).await.unwrap();
            let res = rpc_client.submit_metadata(&msg, 1u32, nonce.clone(), &signature).await;
            match res {
                Ok(_) => {
                    info!("✅ Submit metadata success");
                    nonce += 1;
                },
                Err(e) => {
                    error!("❌ Submit metadata failed: {:?}", e);
                    return;
                },
            }
        } else if let Err(e) = message {
            error!("❗ Error receiving finalized header message: {:?}", e);
        }
    }

    // If the subscription to finalized blocks is disconnected, send an error through the error channel.
    if let Err(error) = error_sender.send(anyhow!("Finalized blocks subscription disconnected")).await {
        error!("🚫 Cannot send error to error channel: {error}");
    }
}
