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

//! # Validator Discovery Mechanism
//!
//! This module provides the functionality for managing and discovering validator nodes in a distributed network,
//! particularly in blockchain and consensus systems. It includes structures and methods for signing validator records,
//! maintaining a cache of validator addresses, and associating peer IDs with validators.

use crate::KademliaKey;
use anyhow::{anyhow, Result};
use codec::{Decode, Encode};
use cumulus_primitives_core::relay_chain::ValidatorId;
use libp2p::{multiaddr::Protocol, multihash::MultihashDigest, Multiaddr, PeerId};
use sp_authority_discovery::{AuthorityId, AuthorityPair, AuthoritySignature};
use sp_core::crypto::{key_types, ByteArray, Pair};
use sp_keystore::Keystore;
use std::collections::{HashMap, HashSet};

/// A signed record containing information about a validator.
///
/// This structure holds serialized data related to a validator, along with a signature
/// and the validator's ID. It can be used to verify the authenticity of the data.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct SignedValidatorRecord {
    pub record: Vec<Vec<u8>>,
    pub validator_id: ValidatorId,
    pub auth_signature: Vec<u8>,
}

impl SignedValidatorRecord {
    /// Generates a Kademlia key based on the validator ID.
    ///
    /// # Arguments
    /// * `validator_id` - The `ValidatorId` for which the key is generated.
    ///
    /// # Returns
    /// A `KademliaKey` derived from the validator's ID.
    pub fn key(validator_id: &ValidatorId) -> KademliaKey {
        KademliaKey::new(&libp2p::multihash::Code::Sha2_256.digest(validator_id.as_ref()).digest())
    }

    /// Verifies the signature of the record.
    ///
    /// This method checks if the stored signature is valid for the serialized record
    /// and the associated validator ID.
    ///
    /// # Returns
    /// `true` if the signature is valid, `false` otherwise.
    pub fn verify_signature(&self) -> bool {
        let signature = AuthoritySignature::decode(&mut self.auth_signature.as_slice())
            .expect("Decode signature failed");
        let public_key = AuthorityId::from_slice(self.validator_id.as_slice())
            .expect("Decode public key failed");

        let message = self.record.iter().flat_map(|v| v.iter()).cloned().collect::<Vec<u8>>();

        AuthorityPair::verify(&signature, &message, &public_key)
    }

    /// Signs a record using the provided keystore and returns a list of signed validator records.
    ///
    /// # Arguments
    /// * `key_store` - A reference to a `Keystore` used for signing.
    /// * `serialized_record` - The serialized data to be signed.
    ///
    /// # Returns
    /// A `Result` containing a vector of tuples, each consisting of a `SignedValidatorRecord` and its corresponding Kademlia key,
    /// or an error if the signing fails.
    pub fn sign_record(
        key_store: &dyn Keystore,
        serialized_record: Vec<Vec<u8>>,
    ) -> Result<Vec<(Self, Vec<u8>)>> {
        let keys = key_store.sr25519_public_keys(key_types::AUTHORITY_DISCOVERY);

        let mut signed_records = Vec::new();

        for key in keys {
            let message =
                serialized_record.iter().flat_map(|v| v.iter()).cloned().collect::<Vec<u8>>();

            let auth_signature = key_store
                .sr25519_sign(key_types::AUTHORITY_DISCOVERY, &key, &message)
                .map_err(|e| anyhow!(e).context(format!("Error signing with key: {:?}", key)))?
                .ok_or_else(|| anyhow!("Could not find key in keystore. Key: {:?}", key))?;

            let auth_signature = auth_signature.encode();

            let signed_record = SignedValidatorRecord {
                record: serialized_record.clone(),
                validator_id: key.clone().into(),
                auth_signature,
            };

            signed_records.push((signed_record, Self::key(&key.into()).as_ref().into()))
        }

        Ok(signed_records)
    }
}

/// A cache structure for storing and retrieving validator addresses and peer IDs.
///
/// This structure maintains mappings between validators' IDs and their associated network addresses,
/// as well as the reverse mapping from peer IDs to validators.
#[derive(Clone, Debug)]
pub struct AddrCache {
    authority_id_to_addresses: HashMap<ValidatorId, HashSet<Multiaddr>>,
    peer_id_to_authority_ids: HashMap<PeerId, HashSet<ValidatorId>>,
}

impl AddrCache {
    /// Creates a new empty `AddrCache`.
    ///
    /// # Returns
    /// A new instance of `AddrCache`.
    pub fn new() -> Self {
        AddrCache {
            authority_id_to_addresses: HashMap::new(),
            peer_id_to_authority_ids: HashMap::new(),
        }
    }

    /// Adds a validator's addresses to the cache.
    ///
    /// This method updates the cache with the addresses associated with a given validator ID.
    /// It also updates the reverse mapping from new peer IDs to the validator ID.
    ///
    /// # Arguments
    /// * `validator_id` - The ID of the validator.
    /// * `addresses` - A vector of `Multiaddr` representing the addresses of the validator.
    pub fn add_validator(&mut self, validator_id: ValidatorId, addresses: Vec<Multiaddr>) {
        let addresses_set = addresses.into_iter().collect::<HashSet<_>>();

        let new_peer_ids = addresses_to_peer_ids(&addresses_set);

        let old_peer_ids = self
            .authority_id_to_addresses
            .get(&validator_id)
            .map(|addresses| addresses_to_peer_ids(addresses))
            .unwrap_or_default();

        self.authority_id_to_addresses.insert(validator_id.clone(), addresses_set);

        for peer_id in new_peer_ids {
            if !old_peer_ids.contains(&peer_id) {
                self.peer_id_to_authority_ids
                    .entry(peer_id)
                    .or_default()
                    .insert(validator_id.clone());
            }
        }
    }

    /// Retrieves the addresses associated with a given validator ID.
    ///
    /// # Arguments
    /// * `validator_id` - The `ValidatorId` for which addresses are to be retrieved.
    ///
    /// # Returns
    /// An `Option` containing a set of `PeerId`s associated with the validator, if any are found.
    pub fn validator_addresses(&self, validator_id: &ValidatorId) -> Option<HashSet<PeerId>> {
        match self.authority_id_to_addresses.get(validator_id) {
            Some(addresses) => Some(addresses_to_peer_ids(addresses)),
            None => None,
        }
    }
}

// Converts a `Multiaddr` to a `PeerId`.
//
// This function extracts the `PeerId` from the last component of a `Multiaddr` if it is of type `P2p`.
fn peer_id_from_multiaddr(addr: &Multiaddr) -> Option<PeerId> {
    addr.iter().last().and_then(|protocol| {
        if let Protocol::P2p(multihash) = protocol {
            PeerId::from_multihash(multihash).ok()
        } else {
            None
        }
    })
}

// Converts a set of `Multiaddr` to a set of `PeerId`.
//
// This function iterates over a set of `Multiaddr` and extracts the `PeerId` from each,
// forming a set of unique `PeerId`s.
fn addresses_to_peer_ids(addresses: &HashSet<Multiaddr>) -> HashSet<PeerId> {
    addresses.iter().filter_map(peer_id_from_multiaddr).collect::<HashSet<_>>()
}
