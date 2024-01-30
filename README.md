# Redot

Redot serves as Polkadot's re-staking layer. It includes a specialized parachain, essentially an adapter for Polkadot's consensus layer, designed to extend the capabilities of the consensus layer. Redot comprises a parachain and redlight. The parachain facilitates on-chain management of various tasks and validators. redlight is a lightweight client for validators, used for completing different validation tasks.

## Philosophy

- Zero Knowledge Leakage: As a consensus layer, Polkadot should not be privy to information about layers like data availability. This focus on consensus allows Polkadot to be more efficient. Redot, as an adapter to the consensus layer, extends its capabilities to the data availability layer through re-staking.
- Neutrality: Redot imposes no restrictions on the nature of tasks. It supports tasks beyond data availability and accommodates various data availability layers.
- Lightweight: Validator tasks should be sufficiently lightweight. Only information pertinent to the consensus layer should be incorporated into these tasks.

## Modules

Redot includes the following modules:

* [core-primitives](./crates/core-primitives/):  Implements specific primitives for DKG, threshold signature encryption.
* [rc-validator](./crates/rc-validator/): Depends on the validator network to execute different methods.
* [rc-validator-fetch](./crates/rc-validator-fetch/): A module that supports storing and fetching validator information in different environments.
* [rc-validator-network](./crates/rc-validator-network/):  Implementation of the validator network for communication among validators.
* [task](./pallets/task/): Task module for managing different tasks and key rotation, etc.
* [validator-registry](./pallets/validator-registry/): Validator registration and deletion module, using OCW for verification.
* [redlight](./redlight/): A lightweight client for validators to complete various validation tasks.
* [redoxt](./redoxt/): A module for communication with the parachain.

## Building

### Setup rust

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

You will also need to install the following packages:

mac

```bash
brew install cmake pkg-config openssl git llvm
```

Linux

```bash
sudo apt install cmake pkg-config libssl-dev git clang libclang-dev protobuf-compiler
```

More: Redot is based on Substrate, for more information please go to [Substrate](https://docs.substrate.io/install/).

### Build

1. Compile the parachain:

```bash
make build-default
```

2. Compile the redlight node:

```bash
make build-light
```

## 3. Run

You can start a development chain with:

```bash
make run-dev
```

However, this does not produce actual blocks. Since Redot is a parachain, it is recommended to use [Zombienet](https://github.com/paritytech/zombienet) for actual operation.

To launch a light node:

```bash
make run-light
```
