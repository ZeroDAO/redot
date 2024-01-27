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

//! Das RPC client.
//! 
//! This is a simple RPC client used for querying the latest block and data availability from DAS (Decentralized Autonomous System).

use serde_json::{json, Value};
use anyhow::{Result, anyhow};

/// A client for interacting with a DAS RPC server.
///
/// This client provides functions to interact with DAS, allowing you to query information
/// such as the latest processed block and check data availability.
pub struct DasClient {
    rpc_url: String,
}

impl DasClient {
    /// Creates a new `DasClient`.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - A string slice that holds the URL of the DAS RPC server.
    pub fn new(rpc_url: String) -> Self {
        DasClient { rpc_url }
    }

    /// Fetches the latest processed block from the DAS system.
    ///
    /// This method queries the DAS RPC server for the most recent block that has been processed.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is either:
    /// - An `Option` containing a tuple of the block number (`u32`) and its hash (`Vec<u8>`), or
    /// - None if the block information is not found or available.
    ///
    /// # Errors
    ///
    /// Returns an error if the request to the RPC server fails, or if the response data
    /// is in an unexpected format.
    pub fn get_latest_block(&self) -> Result<Option<(u32, Vec<u8>)>> {
        let resp = ureq::post(&self.rpc_url)
            .send_json(json!({
                "method": "das_last",
                "params": [],
                "id": 1,
                "jsonrpc": "2.0"
            }))?;

        let value: Value = resp.into_json()?;
        if let Some(result) = value["result"].as_array() {
            let number = result.get(0)
                .and_then(|v| v.as_u64())
                .map(|n| n as u32)
                .ok_or_else(|| anyhow!("Invalid number format"))?;

            let hash_str = result.get(1)
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Invalid hash format"))?;

            let hash = hex::decode(&hash_str.trim_start_matches("0x"))?;

            Ok(Some((number, hash)))
        } else {
            Ok(None)
        }
    }

    /// Checks the data availability for a given block hash in the DAS system.
    ///
    /// Queries the DAS RPC server to check whether the data corresponding to a specific block hash is available.
    ///
    /// # Arguments
    ///
    /// * `block_hash` - A string slice representing the hash of the block to check for data availability.
    ///
    /// # Returns
    ///
    /// Returns a `Result` which is either:
    /// - An `Option` containing a `bool` indicating whether the data is available, or
    /// - None if the availability information is not found or available.
    ///
    /// # Errors
    ///
    /// Returns an error if the request to the RPC server fails, or if the response data
    /// is in an unexpected format.
    pub fn check_data_availability(&self, block_hash: &str) -> Result<Option<bool>> {
        let resp = ureq::post(&self.rpc_url)
            .send_json(json!({
                "method": "das_isAvailable",
                "params": [block_hash],
                "id": 1,
                "jsonrpc": "2.0"
            }))?;
    
        let value: Value = resp.into_json()?;
        match value.get("result") {
            Some(Value::Bool(is_available)) => Ok(Some(*is_available)),
            None => Ok(None),
            _ => Err(anyhow!("Unexpected response format")),
        }
    }
}
