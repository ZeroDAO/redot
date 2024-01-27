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

//! # Validators Information Module
//!
//! This Rust library provides a set of tools and abstractions for managing and updating validator information, 
//! primarily designed for blockchain and distributed ledger technologies. It allows for efficient tracking and 
//! updating of validators based on changes in the blockchain state or external inputs.
//!
//! ## Features
//! - Manage a dynamic set of validators, crucial for blockchain consensus mechanisms.
//! - Store and retrieve validator information from a database, supporting a variety of database implementations through the `DasKv` trait.
//! - Update validators set based on real-time data from a relay chain or blockchain runtime.
//! - Identify new and removed validators, aiding in consensus and governance processes.
//!
//! ## Usage
//! The primary entry point for using this library is the `ValidatorsInfo` struct, which encapsulates the 
//! functionality for handling validators' information. It supports operations such as creating a new 
//! validator set, fetching the current set from the database, updating the set based on external data, 
//! and more.
//!
//! ### Example
//! ```rust,ignore
//! use validators_info_module::ValidatorsInfo;
//! // Assuming `db` is a database instance implementing `DasKv`
//! let mut db = ...;
//!
//! // Creating a new instance of ValidatorsInfo
//! let mut validators_info = ValidatorsInfo::new(&[/* initial validators list */]);
//!
//! // Updating validators information
//! validators_info.update_from_relay(&mut db, /* RelayChainInterface instance */);
//! validators_info.update_from_runtime(&mut db, /* BlockHash */, /* GetValidatorsFromRuntime instance */);
//!
//! // Fetching and storing validators information
//! let validators = validators_info.get(&mut db).unwrap();
//! validators_info.save(&mut db);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

mod info;

pub use info::ValidatorsInfo;